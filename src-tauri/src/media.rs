//! OS media integration: Windows System Media Transport Controls (media keys, the Ally's
//! media overlay, Game Bar) and MPRIS on Linux, via souvlaki.

use crate::player::Player;
use crate::subsonic::Song;
use souvlaki::{MediaControlEvent, MediaControls, MediaMetadata, MediaPlayback, MediaPosition, PlatformConfig};
use std::sync::{Arc, Weak};
use std::time::Duration;

pub struct MediaUpdate {
    pub song: Option<Song>,
    pub cover_url: Option<String>,
    pub paused: bool,
    pub position: f64,
}

/// `hwnd` is the main window handle on Windows (ignored elsewhere).
pub fn start(player: &Arc<Player>, hwnd: Option<isize>) {
    let (tx, rx) = crossbeam_channel::unbounded::<MediaUpdate>();
    let weak: Weak<Player> = Arc::downgrade(player);
    std::thread::Builder::new()
        .name("tunes-media".into())
        .spawn(move || {
            let config = PlatformConfig {
                dbus_name: "tunes",
                display_name: "Tunes",
                hwnd: hwnd.map(|h| h as *mut std::ffi::c_void),
            };
            let mut controls = match MediaControls::new(config) {
                Ok(c) => c,
                Err(e) => {
                    log::warn!("media controls unavailable: {e:?}");
                    return;
                }
            };
            let w = weak.clone();
            if let Err(e) = controls.attach(move |ev| {
                let Some(p) = w.upgrade() else { return };
                match ev {
                    MediaControlEvent::Play => p.play(),
                    MediaControlEvent::Pause | MediaControlEvent::Stop => p.pause(),
                    MediaControlEvent::Toggle => p.toggle(),
                    MediaControlEvent::Next => p.next(),
                    MediaControlEvent::Previous => p.prev(),
                    MediaControlEvent::SetPosition(MediaPosition(d)) => p.seek(d.as_secs_f64()),
                    MediaControlEvent::SeekBy(dir, d) => {
                        let s = d.as_secs_f64();
                        p.seek_relative(if matches!(dir, souvlaki::SeekDirection::Forward) { s } else { -s })
                    }
                    _ => {}
                }
            }) {
                log::warn!("media controls attach failed: {e:?}");
                return;
            }
            let mut current: Option<String> = None;
            while let Ok(u) = rx.recv() {
                let id = u.song.as_ref().map(|s| s.id.clone());
                if id != current {
                    current = id;
                    if let Some(s) = &u.song {
                        let _ = controls.set_metadata(MediaMetadata {
                            title: Some(&s.title),
                            album: s.album.as_deref(),
                            artist: s.artist.as_deref(),
                            cover_url: u.cover_url.as_deref(),
                            duration: s.duration.map(|d| Duration::from_secs(d as u64)),
                        });
                    }
                }
                let progress = Some(MediaPosition(Duration::from_secs_f64(u.position.max(0.0))));
                let _ = controls.set_playback(if u.song.is_none() {
                    MediaPlayback::Stopped
                } else if u.paused {
                    MediaPlayback::Paused { progress }
                } else {
                    MediaPlayback::Playing { progress }
                });
            }
        })
        .expect("spawn media thread");
    player.set_media(tx);
}
