//! Player: queue + transport state on top of the audio engine. Runs independently of the UI.

use crate::audio::decoder::{self, DecCmd, DecoderShared, TrackProvider, TrackReq};
use crate::audio::output::Output;
use crate::config::{app_dir, DirKind};
use crate::subsonic::{Client, Song};
use crossbeam_channel::Sender;
use parking_lot::{Mutex, RwLock};
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};

pub type SharedClient = Arc<RwLock<Option<Client>>>;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum Repeat {
    #[default]
    Off,
    All,
    One,
}

#[derive(Default)]
struct PState {
    queue: Vec<Song>,
    index: usize,
    repeat: Repeat,
    shuffle: bool,
    paused: bool,
    ended: bool,
    /// Nothing has been handed to the decoder yet (e.g. restored session).
    needs_start: bool,
    position_hint: f64,
    /// Audio sequences below this belong to a previous start and are ignored.
    min_seq: u64,
    started_at: Option<Instant>,
    last_seq: u64,
    now_playing_seq: u64,
    scrobbled_seq: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub queue: Vec<Song>,
    pub index: usize,
    pub paused: bool,
    pub ended: bool,
    pub position: f64,
    pub duration: f64,
    pub volume: f32,
    pub repeat: Repeat,
    pub shuffle: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    pub index: usize,
    pub position: f64,
    pub duration: f64,
    pub paused: bool,
    pub volume: f32,
}

#[derive(Serialize, Deserialize, Default)]
struct SavedSession {
    queue: Vec<Song>,
    index: usize,
    position: f64,
    repeat: Repeat,
    shuffle: bool,
}

struct Provider {
    state: Arc<Mutex<PState>>,
    client: SharedClient,
}

impl TrackProvider for Provider {
    fn track_at(&self, index: usize) -> Option<TrackReq> {
        let song = self.state.lock().queue.get(index)?.clone();
        let c = self.client.read();
        let c = c.as_ref()?;
        Some(TrackReq {
            raw_url: c.stream_url(&song.id, false),
            transcode_url: c.stream_url(&song.id, true),
            suffix: song.suffix.clone().unwrap_or_default(),
            song_id: song.id,
        })
    }

    fn next_index(&self, index: usize) -> Option<usize> {
        let s = self.state.lock();
        next_of(&s, index)
    }
}

fn next_of(s: &PState, index: usize) -> Option<usize> {
    match s.repeat {
        Repeat::One => Some(index),
        _ if index + 1 < s.queue.len() => Some(index + 1),
        Repeat::All if !s.queue.is_empty() => Some(0),
        _ => None,
    }
}

pub struct Player {
    state: Arc<Mutex<PState>>,
    out: Output,
    dec_tx: Sender<DecCmd>,
    dec: Arc<DecoderShared>,
    client: SharedClient,
    app: AppHandle,
    media: Mutex<Option<Sender<crate::media::MediaUpdate>>>,
    on_volume: Box<dyn Fn(f32) + Send + Sync>,
}

impl Player {
    pub fn new(app: AppHandle, client: SharedClient, volume: f32, on_volume: impl Fn(f32) + Send + Sync + 'static) -> Arc<Self> {
        let state = Arc::new(Mutex::new(PState { paused: true, ..Default::default() }));
        let out = Output::start(volume);
        let (dec_tx, dec_rx) = crossbeam_channel::unbounded();
        let provider = Arc::new(Provider { state: state.clone(), client: client.clone() });
        let dec = decoder::spawn(out.shared.clone(), provider, dec_rx);
        let p = Arc::new(Self {
            state,
            out,
            dec_tx,
            dec,
            client,
            app,
            media: Mutex::new(None),
            on_volume: Box::new(on_volume),
        });
        p.restore_session();
        let weak = Arc::downgrade(&p);
        std::thread::Builder::new()
            .name("tunes-ticker".into())
            .spawn(move || {
                let mut last_save = Instant::now();
                loop {
                    std::thread::sleep(Duration::from_millis(250));
                    let Some(p) = weak.upgrade() else { return };
                    p.tick();
                    if last_save.elapsed() > Duration::from_secs(15) {
                        p.save_session();
                        last_save = Instant::now();
                    }
                }
            })
            .expect("spawn ticker");
        p
    }

    pub fn set_media(&self, tx: Sender<crate::media::MediaUpdate>) {
        *self.media.lock() = Some(tx);
        self.push_media();
    }

    fn position_locked(&self, s: &PState) -> (f64, Option<usize>) {
        let (seq, frames) = {
            let g = self.out.shared.inner.lock();
            (g.seq, g.frames)
        };
        if s.needs_start || seq == 0 || seq < s.min_seq {
            return (s.position_hint, None);
        }
        let rate = self.out.shared.rate.load(Ordering::Relaxed).max(1) as f64;
        match self.dec.seqs.lock().get(&seq) {
            Some(info) => (info.offset + frames as f64 / rate, Some(info.index)),
            None => (s.position_hint, None),
        }
    }

    fn tick(&self) {
        let mut changed = false;
        let mut scrobble: Option<(String, bool)> = None;
        let progress;
        {
            let mut s = self.state.lock();
            let audible_seq = self.out.shared.inner.lock().seq;
            if audible_seq != 0 && audible_seq >= s.min_seq && audible_seq != s.last_seq {
                if let Some(info) = self.dec.seqs.lock().get(&audible_seq).cloned() {
                    s.last_seq = audible_seq;
                    if info.index < s.queue.len() {
                        if s.index != info.index {
                            changed = true;
                        }
                        s.index = info.index;
                    }
                    s.position_hint = info.offset;
                }
            }
            let (pos, _) = self.position_locked(&s);
            let dur = s.queue.get(s.index).and_then(|x| x.duration).unwrap_or(0) as f64;

            // Now-playing + scrobble submission (Navidrome builds "Recently Played" from these).
            if !s.paused && s.last_seq != 0 {
                if let Some(song_id) = s.queue.get(s.index).map(|x| x.id.clone()) {
                    if s.now_playing_seq != s.last_seq {
                        s.now_playing_seq = s.last_seq;
                        scrobble = Some((song_id, false));
                        changed = true;
                    } else if s.scrobbled_seq != s.last_seq && dur > 0.0 && pos >= (dur / 2.0).min(240.0) {
                        s.scrobbled_seq = s.last_seq;
                        scrobble = Some((song_id, true));
                    }
                }
            }

            // End of queue.
            if !s.paused
                && !s.needs_start
                && (s.last_seq >= s.min_seq || s.started_at.is_some_and(|t| t.elapsed() > Duration::from_secs(10)))
                && self.dec.idle.load(Ordering::SeqCst)
                && self.out.shared.queued.load(Ordering::Relaxed) == 0
            {
                s.paused = true;
                s.ended = true;
                s.needs_start = true;
                s.position_hint = 0.0;
                self.out.pause();
                changed = true;
            }
            progress = Progress {
                index: s.index,
                position: pos,
                duration: dur,
                paused: s.paused,
                volume: self.out.shared.volume(),
            };
        }
        if let Some((id, submission)) = scrobble {
            if let Some(c) = self.client.read().clone() {
                tauri::async_runtime::spawn(async move {
                    if let Err(e) = c.scrobble(&id, submission).await {
                        log::warn!("scrobble failed: {e}");
                    }
                });
            }
        }
        if changed {
            self.emit_state();
            self.save_session();
        } else if !progress.paused {
            let _ = self.app.emit("player-progress", &progress);
        }
    }

    pub fn snapshot(&self) -> Snapshot {
        let s = self.state.lock();
        let (pos, _) = self.position_locked(&s);
        Snapshot {
            queue: s.queue.clone(),
            index: s.index,
            paused: s.paused,
            ended: s.ended,
            position: pos,
            duration: s.queue.get(s.index).and_then(|x| x.duration).unwrap_or(0) as f64,
            volume: self.out.shared.volume(),
            repeat: s.repeat,
            shuffle: s.shuffle,
            error: self.dec.error.lock().clone(),
        }
    }

    fn emit_state(&self) {
        let _ = self.app.emit("player-state", self.snapshot());
        self.push_media();
    }

    fn push_media(&self) {
        let Some(tx) = self.media.lock().clone() else { return };
        let snap = self.snapshot();
        let song = snap.queue.get(snap.index).cloned();
        let cover = song.as_ref().and_then(|s| s.cover_art.clone()).and_then(|id| {
            self.client.read().as_ref().map(|c| c.url("getCoverArt", &[("id", &id), ("size", "512")]))
        });
        let _ = tx.send(crate::media::MediaUpdate { song, cover_url: cover, paused: snap.paused, position: snap.position });
    }

    fn start_at(&self, s: &mut PState, index: usize, secs: f64) {
        s.index = index;
        s.position_hint = secs;
        s.needs_start = false;
        s.ended = false;
        s.paused = false;
        // Ignore stale audio sequences until the new one becomes audible.
        s.min_seq = self.dec.seqs.lock().keys().copied().max().unwrap_or(0) + 1;
        s.started_at = Some(Instant::now());
        let _ = self.dec_tx.send(DecCmd::Start { index, start_secs: secs });
        self.out.play();
    }

    // ---- public transport API -------------------------------------------------------------

    pub fn play_queue(&self, songs: Vec<Song>, index: usize) {
        if songs.is_empty() {
            return;
        }
        {
            let mut s = self.state.lock();
            s.queue = songs;
            if s.shuffle {
                let at = index.min(s.queue.len() - 1);
                let first = s.queue.remove(at);
                s.queue.shuffle(&mut rand::thread_rng());
                s.queue.insert(0, first);
                self.start_at(&mut s, 0, 0.0);
            } else {
                let i = index.min(s.queue.len() - 1);
                self.start_at(&mut s, i, 0.0);
            }
        }
        self.emit_state();
    }

    pub fn enqueue(&self, songs: Vec<Song>, play_next: bool) {
        {
            let mut s = self.state.lock();
            if s.queue.is_empty() {
                s.queue = songs;
                self.start_at(&mut s, 0, 0.0);
            } else {
                let at = if play_next { (s.index + 1).min(s.queue.len()) } else { s.queue.len() };
                s.queue.splice(at..at, songs);
            }
        }
        self.emit_state();
    }

    pub fn toggle(&self) {
        let paused = self.state.lock().paused;
        if paused {
            self.play()
        } else {
            self.pause()
        }
    }

    pub fn play(&self) {
        {
            let mut s = self.state.lock();
            if s.queue.is_empty() {
                return;
            }
            if s.needs_start {
                let (i, p) = (s.index, if s.ended { 0.0 } else { s.position_hint });
                self.start_at(&mut s, i, p);
            } else {
                s.paused = false;
                self.out.play();
            }
        }
        self.emit_state();
    }

    pub fn pause(&self) {
        {
            let mut s = self.state.lock();
            s.paused = true;
            let (pos, _) = self.position_locked(&s);
            s.position_hint = pos;
            self.out.pause();
        }
        self.emit_state();
        self.save_session();
    }

    pub fn next(&self) {
        {
            let mut s = self.state.lock();
            let target = match s.repeat {
                Repeat::One => {
                    // Manual skip should still advance even in repeat-one.
                    if s.index + 1 < s.queue.len() { Some(s.index + 1) } else if s.queue.is_empty() { None } else { Some(0) }
                }
                _ => next_of(&s, s.index),
            };
            match target {
                Some(i) => self.start_at(&mut s, i, 0.0),
                None => return,
            }
        }
        self.emit_state();
    }

    pub fn prev(&self) {
        {
            let mut s = self.state.lock();
            if s.queue.is_empty() {
                return;
            }
            let (pos, _) = self.position_locked(&s);
            let i = if pos > 3.0 || s.index == 0 { s.index } else { s.index - 1 };
            self.start_at(&mut s, i, 0.0);
        }
        self.emit_state();
    }

    pub fn jump(&self, index: usize) {
        {
            let mut s = self.state.lock();
            if index >= s.queue.len() {
                return;
            }
            self.start_at(&mut s, index, 0.0);
        }
        self.emit_state();
    }

    pub fn seek(&self, secs: f64) {
        {
            let mut s = self.state.lock();
            if s.queue.is_empty() {
                return;
            }
            let i = s.index;
            let was_paused = s.paused;
            self.start_at(&mut s, i, secs.max(0.0));
            if was_paused {
                s.paused = true;
                self.out.pause();
            }
        }
        self.emit_state();
    }

    pub fn seek_relative(&self, delta: f64) {
        let pos = {
            let s = self.state.lock();
            self.position_locked(&s).0
        };
        self.seek(pos + delta);
    }

    pub fn set_volume(&self, v: f32) -> f32 {
        let v = (v.clamp(0.0, 1.0) * 100.0).round() / 100.0;
        self.out.shared.set_volume(v);
        (self.on_volume)(v);
        let _ = self.app.emit("player-volume", v);
        v
    }

    pub fn change_volume(&self, delta: f32) -> f32 {
        self.set_volume(self.out.shared.volume() + delta)
    }

    pub fn set_repeat(&self, r: Repeat) {
        self.state.lock().repeat = r;
        self.emit_state();
    }

    pub fn toggle_shuffle(&self) {
        {
            let mut s = self.state.lock();
            s.shuffle = !s.shuffle;
            if s.shuffle && s.index + 1 < s.queue.len() {
                let start = s.index + 1;
                s.queue[start..].shuffle(&mut rand::thread_rng());
            }
        }
        self.emit_state();
    }

    // ---- session persistence ------------------------------------------------------------

    fn session_path() -> std::path::PathBuf {
        app_dir(DirKind::Config).join("session.json")
    }

    pub fn save_session(&self) {
        let saved = {
            let s = self.state.lock();
            let (pos, _) = self.position_locked(&s);
            SavedSession { queue: s.queue.clone(), index: s.index, position: pos, repeat: s.repeat, shuffle: s.shuffle }
        };
        if let Ok(b) = serde_json::to_vec(&saved) {
            let path = Self::session_path();
            let tmp = path.with_extension("tmp");
            if std::fs::write(&tmp, b).is_ok() {
                let _ = std::fs::rename(tmp, path);
            }
        }
    }

    fn restore_session(&self) {
        let Some(saved) = std::fs::read(Self::session_path())
            .ok()
            .and_then(|b| serde_json::from_slice::<SavedSession>(&b).ok())
        else {
            return;
        };
        let mut s = self.state.lock();
        s.index = saved.index.min(saved.queue.len().saturating_sub(1));
        s.queue = saved.queue;
        s.position_hint = saved.position;
        s.repeat = saved.repeat;
        s.shuffle = saved.shuffle;
        s.paused = true;
        s.needs_start = true;
    }
}
