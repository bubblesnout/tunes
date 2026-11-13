//! Decoder thread: pulls tracks from the queue, decodes with symphonia, resamples to the
//! device rate and feeds the output. Decodes straight into the next track for gapless playback.

use super::output::Shared as OutShared;
use super::stream::{CancelHandle, HttpSource};
use crossbeam_channel::{Receiver, TryRecvError};
use parking_lot::Mutex;
use rubato::{Resampler, SincFixedIn, SincInterpolationParameters, SincInterpolationType, WindowFunction};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::errors::Error as SymError;
use symphonia::core::formats::{FormatOptions, SeekMode, SeekTo};
use symphonia::core::io::{MediaSourceStream, MediaSourceStreamOptions};
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use symphonia::core::units::Time;

/// Keep this much decoded audio queued ahead of the device.
const TARGET_BUFFER_SECS: f32 = 1.5;
const RESAMPLE_CHUNK: usize = 1024;

/// Codecs symphonia can't decode: ask the server to transcode to MP3 instead.
const TRANSCODE_SUFFIXES: &[&str] = &["opus", "wma", "ape", "wv", "mpc", "dsf", "dff", "ac3", "dts", "tta"];

#[derive(Clone)]
pub struct TrackReq {
    pub song_id: String,
    pub suffix: String,
    pub raw_url: String,
    pub transcode_url: String,
}

/// Lets the decoder ask the player which track comes next (queue can change while playing).
pub trait TrackProvider: Send + Sync + 'static {
    fn track_at(&self, index: usize) -> Option<TrackReq>;
    fn next_index(&self, index: usize) -> Option<usize>;
}

#[derive(Clone, Debug)]
pub struct SeqInfo {
    pub index: usize,
    pub song_id: String,
    /// Track position (secs) at which this seq began (non-zero after a seek).
    pub offset: f64,
}

pub enum DecCmd {
    Start { index: usize, start_secs: f64 },
    Stop,
}

pub struct DecoderShared {
    pub seqs: Mutex<HashMap<u64, SeqInfo>>,
    pub idle: AtomicBool,
    pub error: Mutex<Option<String>>,
}

enum Outcome {
    Finished,
    Interrupted(DecCmd),
    Failed(String),
}

pub fn spawn(out: Arc<OutShared>, provider: Arc<dyn TrackProvider>, rx: Receiver<DecCmd>) -> Arc<DecoderShared> {
    let shared = Arc::new(DecoderShared {
        seqs: Mutex::new(HashMap::new()),
        idle: AtomicBool::new(true),
        error: Mutex::new(None),
    });
    let s = shared.clone();
    std::thread::Builder::new()
        .name("tunes-decoder".into())
        .spawn(move || run(out, provider, rx, s))
        .expect("spawn decoder");
    shared
}

fn run(out: Arc<OutShared>, provider: Arc<dyn TrackProvider>, rx: Receiver<DecCmd>, shared: Arc<DecoderShared>) {
    let mut seq: u64 = 0;
    let mut pending: Option<DecCmd> = None;
    loop {
        let cmd = match pending.take() {
            Some(c) => c,
            None => match rx.recv() {
                Ok(c) => c,
                Err(_) => return,
            },
        };
        let (mut index, mut start) = match cmd {
            DecCmd::Stop => {
                out.flush();
                shared.idle.store(true, Ordering::SeqCst);
                continue;
            }
            DecCmd::Start { index, start_secs } => (index, start_secs),
        };
        out.flush();
        shared.idle.store(false, Ordering::SeqCst);
        *shared.error.lock() = None;
        let mut failures = 0;
        loop {
            let Some(track) = provider.track_at(index) else { break };
            seq += 1;
            {
                let mut map = shared.seqs.lock();
                // Forget old sequences so the map doesn't grow forever.
                map.retain(|k, _| *k + 8 > seq);
                map.insert(seq, SeqInfo { index, song_id: track.song_id.clone(), offset: start });
            }
            match play_track(&out, &track, seq, start, &rx, &shared) {
                Outcome::Finished => failures = 0,
                Outcome::Interrupted(c) => {
                    pending = Some(c);
                    break;
                }
                Outcome::Failed(e) => {
                    log::warn!("track {} failed: {e}", track.song_id);
                    *shared.error.lock() = Some(e);
                    failures += 1;
                    if failures >= 3 {
                        break;
                    }
                }
            }
            start = 0.0;
            match provider.next_index(index) {
                Some(n) => index = n,
                None => break,
            }
        }
        if pending.is_none() {
            shared.idle.store(true, Ordering::SeqCst);
        }
    }
}

fn open(url: &str, suffix: &str) -> anyhow::Result<(Box<dyn symphonia::core::formats::FormatReader>, CancelHandle)> {
    let (src, cancel) = HttpSource::open(url)?;
    let mss = MediaSourceStream::new(Box::new(src), MediaSourceStreamOptions { buffer_len: 256 * 1024 });
    let mut hint = Hint::new();
    if !suffix.is_empty() {
        hint.with_extension(suffix);
    }
    let probed = symphonia::default::get_probe().format(
        &hint,
        mss,
        &FormatOptions { enable_gapless: true, ..Default::default() },
        &MetadataOptions::default(),
    )?;
    Ok((probed.format, cancel))
}

fn play_track(
    out: &Arc<OutShared>,
    track: &TrackReq,
    seq: u64,
    start_secs: f64,
    rx: &Receiver<DecCmd>,
    shared: &Arc<DecoderShared>,
) -> Outcome {
    let suffix = track.suffix.to_lowercase();
    let force_transcode = TRANSCODE_SUFFIXES.contains(&suffix.as_str());
    // Try the original file first; fall back to server-side MP3 if we can't decode it.
    let attempts: Vec<(&str, &str)> = if force_transcode {
        vec![(&track.transcode_url, "mp3")]
    } else {
        vec![(&track.raw_url, suffix.as_str()), (&track.transcode_url, "mp3")]
    };
    let mut last_err = String::new();
    for (url, hint) in attempts {
        let (mut format, cancel) = match open(url, hint) {
            Ok(f) => f,
            Err(e) => {
                last_err = e.to_string();
                continue;
            }
        };
        let Some(t) = format.tracks().iter().find(|t| t.codec_params.codec != CODEC_TYPE_NULL).cloned() else {
            last_err = "no audio track".into();
            continue;
        };
        let mut decoder = match symphonia::default::get_codecs().make(&t.codec_params, &DecoderOptions::default()) {
            Ok(d) => d,
            Err(e) => {
                last_err = e.to_string();
                cancel.cancel();
                continue;
            }
        };
        if start_secs > 0.5 {
            match format.seek(
                SeekMode::Coarse,
                SeekTo::Time { time: Time::from(start_secs), track_id: Some(t.id) },
            ) {
                Ok(_) => decoder.reset(),
                Err(e) => log::warn!("seek failed: {e}"),
            }
        }
        let result = decode_loop(out, &mut *format, &mut *decoder, t.id, seq, rx, shared);
        cancel.cancel();
        return result;
    }
    Outcome::Failed(last_err)
}

fn decode_loop(
    out: &Arc<OutShared>,
    format: &mut dyn symphonia::core::formats::FormatReader,
    decoder: &mut dyn symphonia::core::codecs::Decoder,
    track_id: u32,
    seq: u64,
    rx: &Receiver<DecCmd>,
    _shared: &Arc<DecoderShared>,
) -> Outcome {
    let out_rate = out.rate.load(Ordering::SeqCst) as usize;
    let out_ch = out.channels.load(Ordering::SeqCst) as usize;
    let mut conv: Option<Converter> = None;
    let mut sample_buf: Option<SampleBuffer<f32>> = None;

    loop {
        // React to user commands promptly.
        match rx.try_recv() {
            Ok(c) => return Outcome::Interrupted(c),
            Err(TryRecvError::Disconnected) => return Outcome::Finished,
            Err(TryRecvError::Empty) => {}
        }
        // Don't run too far ahead of the device.
        while out.buffered_secs() > TARGET_BUFFER_SECS {
            match rx.recv_timeout(Duration::from_millis(50)) {
                Ok(c) => return Outcome::Interrupted(c),
                Err(crossbeam_channel::RecvTimeoutError::Disconnected) => return Outcome::Finished,
                Err(crossbeam_channel::RecvTimeoutError::Timeout) => {}
            }
        }

        let packet = match format.next_packet() {
            Ok(p) => p,
            Err(SymError::IoError(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(SymError::IoError(e)) => {
                // Cancelled/truncated stream: treat what we have as the end of the track.
                log::warn!("stream io error: {e}");
                break;
            }
            Err(SymError::ResetRequired) => {
                decoder.reset();
                continue;
            }
            Err(e) => return Outcome::Failed(e.to_string()),
        };
        if packet.track_id() != track_id {
            continue;
        }
        let decoded = match decoder.decode(&packet) {
            Ok(d) => d,
            Err(SymError::DecodeError(e)) => {
                log::debug!("decode error (skipping packet): {e}");
                continue;
            }
            Err(SymError::IoError(_)) => break,
            Err(e) => return Outcome::Failed(e.to_string()),
        };
        let spec = *decoded.spec();
        let frames = decoded.frames();
        if frames == 0 {
            continue;
        }
        let sb = match &mut sample_buf {
            Some(sb) if sb.capacity() >= frames * spec.channels.count() => sb,
            _ => sample_buf.insert(SampleBuffer::<f32>::new(decoded.capacity() as u64, spec)),
        };
        sb.copy_interleaved_ref(decoded);
        let c = conv.get_or_insert_with(|| Converter::new(spec.rate as usize, out_rate, spec.channels.count(), out_ch));
        let pcm = c.process(sb.samples());
        out.push(seq, pcm);
    }
    if let Some(mut c) = conv {
        out.push(seq, c.finish());
    }
    Outcome::Finished
}

/// Sample-rate + channel-layout conversion to the output device format.
struct Converter {
    in_ch: usize,
    out_ch: usize,
    resampler: Option<SincFixedIn<f32>>,
    pending: Vec<Vec<f32>>,
}

impl Converter {
    fn new(in_rate: usize, out_rate: usize, in_ch: usize, out_ch: usize) -> Self {
        let resampler = (in_rate != out_rate).then(|| {
            let params = SincInterpolationParameters {
                sinc_len: 128,
                f_cutoff: 0.95,
                interpolation: SincInterpolationType::Linear,
                oversampling_factor: 128,
                window: WindowFunction::BlackmanHarris2,
            };
            SincFixedIn::<f32>::new(out_rate as f64 / in_rate as f64, 1.0, params, RESAMPLE_CHUNK, in_ch)
                .expect("resampler")
        });
        Self { in_ch, out_ch, resampler, pending: vec![Vec::new(); in_ch] }
    }

    fn process(&mut self, interleaved: &[f32]) -> Vec<f32> {
        let Some(rs) = self.resampler.as_mut() else {
            return remap(interleaved, self.in_ch, self.out_ch);
        };
        for frame in interleaved.chunks_exact(self.in_ch) {
            for (c, s) in frame.iter().enumerate() {
                self.pending[c].push(*s);
            }
        }
        let mut result = Vec::new();
        while self.pending[0].len() >= rs.input_frames_next() {
            let need = rs.input_frames_next();
            let input: Vec<&[f32]> = self.pending.iter().map(|ch| &ch[..need]).collect();
            if let Ok(o) = rs.process(&input, None) {
                result.extend(interleave_remap(&o, self.out_ch));
            }
            for ch in &mut self.pending {
                ch.drain(..need);
            }
        }
        result
    }

    fn finish(&mut self) -> Vec<f32> {
        let Some(rs) = self.resampler.as_mut() else { return Vec::new() };
        if self.pending[0].is_empty() {
            return Vec::new();
        }
        let input: Vec<&[f32]> = self.pending.iter().map(|c| c.as_slice()).collect();
        let out = rs.process_partial(Some(&input), None).map(|o| interleave_remap(&o, self.out_ch)).unwrap_or_default();
        self.pending.iter_mut().for_each(Vec::clear);
        out
    }
}

fn map_channel(frame_in: &[f32], out_ch: usize, dst: &mut Vec<f32>) {
    let in_ch = frame_in.len();
    for oc in 0..out_ch {
        let s = if in_ch == 1 {
            if oc < 2 { frame_in[0] } else { 0.0 }
        } else if oc < in_ch {
            frame_in[oc]
        } else {
            0.0
        };
        dst.push(s);
    }
}

fn remap(interleaved: &[f32], in_ch: usize, out_ch: usize) -> Vec<f32> {
    if in_ch == out_ch {
        return interleaved.to_vec();
    }
    let mut v = Vec::with_capacity(interleaved.len() / in_ch * out_ch);
    for f in interleaved.chunks_exact(in_ch) {
        map_channel(f, out_ch, &mut v);
    }
    v
}

fn interleave_remap(planar: &[Vec<f32>], out_ch: usize) -> Vec<f32> {
    let frames = planar.first().map_or(0, Vec::len);
    let mut v = Vec::with_capacity(frames * out_ch);
    let mut frame = vec![0.0; planar.len()];
    for i in 0..frames {
        for (c, ch) in planar.iter().enumerate() {
            frame[c] = ch[i];
        }
        map_channel(&frame, out_ch, &mut v);
    }
    v
}
