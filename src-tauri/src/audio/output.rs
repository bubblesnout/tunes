//! Audio output via cpal. Decoded, already-resampled interleaved f32 chunks are queued here;
//! the device callback applies the in-app volume (never touches system volume).

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, SizedSample, FromSample};
use crossbeam_channel::{Receiver, Sender};
use parking_lot::Mutex;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

pub struct Chunk {
    pub seq: u64,
    pub data: Vec<f32>,
    pub pos: usize,
}

#[derive(Default)]
pub struct Inner {
    pub chunks: VecDeque<Chunk>,
    /// Sequence id (track instance) currently audible, and frames of it played so far.
    pub seq: u64,
    pub frames: u64,
}

pub struct Shared {
    pub inner: Mutex<Inner>,
    pub queued: AtomicUsize,
    pub paused: AtomicBool,
    volume: AtomicU32,
    pub rate: AtomicU32,
    pub channels: AtomicU32,
}

impl Shared {
    pub fn volume(&self) -> f32 {
        f32::from_bits(self.volume.load(Ordering::Relaxed))
    }
    pub fn set_volume(&self, v: f32) {
        self.volume.store(v.clamp(0.0, 1.0).to_bits(), Ordering::Relaxed);
    }
    pub fn push(&self, seq: u64, data: Vec<f32>) {
        if data.is_empty() {
            return;
        }
        self.queued.fetch_add(data.len(), Ordering::Relaxed);
        self.inner.lock().chunks.push_back(Chunk { seq, data, pos: 0 });
    }
    pub fn flush(&self) {
        let mut g = self.inner.lock();
        g.chunks.clear();
        self.queued.store(0, Ordering::Relaxed);
    }
    /// Seconds of audio waiting in the queue.
    pub fn buffered_secs(&self) -> f32 {
        let rate = self.rate.load(Ordering::Relaxed).max(1) as f32;
        let ch = self.channels.load(Ordering::Relaxed).max(1) as f32;
        self.queued.load(Ordering::Relaxed) as f32 / (rate * ch)
    }
}

pub enum OutCmd {
    Play,
    Pause,
}

pub struct Output {
    pub shared: Arc<Shared>,
    tx: Sender<OutCmd>,
}

impl Output {
    pub fn start(volume: f32) -> Self {
        let shared = Arc::new(Shared {
            inner: Mutex::new(Inner::default()),
            queued: AtomicUsize::new(0),
            paused: AtomicBool::new(true),
            volume: AtomicU32::new(volume.clamp(0.0, 1.0).to_bits()),
            rate: AtomicU32::new(48_000),
            channels: AtomicU32::new(2),
        });
        let (tx, rx) = crossbeam_channel::unbounded();
        let (ready_tx, ready_rx) = crossbeam_channel::bounded(1);
        let s = shared.clone();
        std::thread::Builder::new()
            .name("tunes-output".into())
            .spawn(move || output_thread(s, rx, ready_tx))
            .expect("spawn output thread");
        // Wait briefly so rate/channels are known before decoding begins.
        let _ = ready_rx.recv_timeout(Duration::from_secs(3));
        Self { shared, tx }
    }

    pub fn play(&self) {
        self.shared.paused.store(false, Ordering::SeqCst);
        let _ = self.tx.send(OutCmd::Play);
    }

    pub fn pause(&self) {
        self.shared.paused.store(true, Ordering::SeqCst);
        let _ = self.tx.send(OutCmd::Pause);
    }
}

fn output_thread(shared: Arc<Shared>, rx: Receiver<OutCmd>, ready: Sender<()>) {
    let mut ready = Some(ready);
    loop {
        let failed = Arc::new(AtomicBool::new(false));
        let stream = match build_stream(&shared, failed.clone()) {
            Ok(s) => s,
            Err(e) => {
                log::error!("audio output unavailable: {e}");
                if let Some(r) = ready.take() {
                    let _ = r.send(());
                }
                std::thread::sleep(Duration::from_secs(2));
                continue;
            }
        };
        if let Some(r) = ready.take() {
            let _ = r.send(());
        }
        if !shared.paused.load(Ordering::SeqCst) {
            let _ = stream.play();
        }
        // Service play/pause commands until the device fails (e.g. headphones unplugged).
        loop {
            match rx.recv_timeout(Duration::from_millis(500)) {
                Ok(OutCmd::Play) => {
                    let _ = stream.play();
                }
                Ok(OutCmd::Pause) => {
                    let _ = stream.pause();
                }
                Err(crossbeam_channel::RecvTimeoutError::Timeout) => {}
                Err(crossbeam_channel::RecvTimeoutError::Disconnected) => return,
            }
            if failed.load(Ordering::SeqCst) {
                log::warn!("audio device lost, reopening default device");
                break;
            }
        }
        drop(stream);
        std::thread::sleep(Duration::from_millis(500));
    }
}

fn build_stream(shared: &Arc<Shared>, failed: Arc<AtomicBool>) -> anyhow::Result<cpal::Stream> {
    let host = cpal::default_host();
    let device = host.default_output_device().ok_or_else(|| anyhow::anyhow!("no output device"))?;
    let supported = device.default_output_config()?;
    let format = supported.sample_format();
    let config: cpal::StreamConfig = supported.into();
    shared.rate.store(config.sample_rate.0, Ordering::SeqCst);
    shared.channels.store(config.channels as u32, Ordering::SeqCst);
    match format {
        SampleFormat::F32 => make::<f32>(&device, &config, shared.clone(), failed),
        SampleFormat::I16 => make::<i16>(&device, &config, shared.clone(), failed),
        SampleFormat::U16 => make::<u16>(&device, &config, shared.clone(), failed),
        SampleFormat::I32 => make::<i32>(&device, &config, shared.clone(), failed),
        other => anyhow::bail!("unsupported sample format {other:?}"),
    }
}

fn make<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    shared: Arc<Shared>,
    failed: Arc<AtomicBool>,
) -> anyhow::Result<cpal::Stream>
where
    T: SizedSample + FromSample<f32>,
{
    let channels = config.channels as usize;
    let mut gain = shared.volume() * shared.volume();
    let stream = device.build_output_stream(
        config,
        move |out: &mut [T], _| {
            let target = {
                let v = shared.volume();
                v * v // perceptual-ish curve
            };
            let paused = shared.paused.load(Ordering::Relaxed);
            let Some(mut g) = (if paused { None } else { shared.inner.try_lock() }) else {
                out.iter_mut().for_each(|s| *s = T::from_sample(0.0));
                return;
            };
            let n = out.len();
            let step = (target - gain) / n.max(1) as f32;
            let mut i = 0;
            let mut consumed = 0usize;
            while i < n {
                let Some(seq) = g.chunks.front().map(|c| c.seq) else { break };
                if seq != g.seq {
                    g.seq = seq;
                    g.frames = 0;
                    continue;
                }
                let front = g.chunks.front_mut().unwrap();
                let take = (n - i).min(front.data.len() - front.pos);
                for k in 0..take {
                    gain += step;
                    out[i + k] = T::from_sample(front.data[front.pos + k] * gain);
                }
                front.pos += take;
                i += take;
                consumed += take;
                if front.pos >= front.data.len() {
                    g.chunks.pop_front();
                }
                g.frames += (take / channels) as u64;
            }
            gain = target;
            drop(g);
            if consumed > 0 {
                shared.queued.fetch_sub(consumed, Ordering::Relaxed);
            }
            for s in &mut out[i..] {
                *s = T::from_sample(0.0);
            }
        },
        move |err| {
            log::error!("audio stream error: {err}");
            failed.store(true, Ordering::SeqCst);
        },
        None,
    )?;
    Ok(stream)
}
