//! Persistent, non-secret settings stored as JSON in the platform config dir.

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct HomeSection {
    pub id: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Config {
    pub server_url: String,
    pub username: String,
    pub home_sections: Vec<HomeSection>,
    /// Library pages shown in the main menu under Home, in order.
    pub menu_sections: Vec<HomeSection>,
    /// What the Artists page lists: "albumArtist", "artist" (track artists) or "both"
    /// (two pages in the main menu: Album Artists and Artists).
    pub artists_view: String,
    /// action -> list of inputs (keyboard `Key:<code>` or gamepad `Pad:<button>`)
    pub bindings: serde_json::Value,
    pub volume: f32,
    pub theme: String,
    pub fullscreen: bool,
    /// Run the Steam overlay host (`overlay_host.rs`) when launched from Steam.
    pub steam_overlay: bool,
}

impl Default for HomeSection {
    fn default() -> Self {
        Self { id: String::new(), enabled: true }
    }
}

pub const ALL_SECTIONS: &[&str] = &[
    "resume", "recentlyAdded", "recentlyPlayed", "mostPlayed", "starred", "random", "highestRated", "playlists",
];
const DEFAULT_ON: &[&str] = &["resume", "recentlyAdded", "recentlyPlayed", "mostPlayed", "random"];

pub const ARTISTS_VIEWS: &[&str] = &["albumArtist", "artist", "both"];

pub const ALL_MENU_SECTIONS: &[&str] = &["artists", "albums", "playlists", "genres", "favourites"];
const MENU_DEFAULT_ON: &[&str] = &["artists", "albums", "playlists", "favourites"];

fn sections(all: &[&str], on: &[&str]) -> Vec<HomeSection> {
    all.iter().map(|s| HomeSection { id: s.to_string(), enabled: on.contains(s) }).collect()
}

/// Keep known ids once each, in the saved order; append any new ones with the given default state.
fn normalise_sections(list: &mut Vec<HomeSection>, all: &[&str], on: &[&str]) {
    list.retain(|s| all.contains(&s.id.as_str()));
    let mut seen = std::collections::HashSet::new();
    list.retain(|s| seen.insert(s.id.clone()));
    for id in all {
        if !list.iter().any(|s| s.id == *id) {
            list.push(HomeSection { id: id.to_string(), enabled: on.contains(id) });
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server_url: String::new(),
            username: String::new(),
            home_sections: sections(ALL_SECTIONS, DEFAULT_ON),
            menu_sections: sections(ALL_MENU_SECTIONS, MENU_DEFAULT_ON),
            artists_view: "albumArtist".into(),
            bindings: serde_json::Value::Null,
            volume: 0.8,
            theme: "bigpicture".into(),
            fullscreen: true,
            steam_overlay: true,
        }
    }
}

impl Config {
    /// Ensure every known section appears exactly once. New Home shelves are appended
    /// disabled; new menu pages get their default state.
    fn normalise(&mut self) {
        normalise_sections(&mut self.home_sections, ALL_SECTIONS, &[]);
        normalise_sections(&mut self.menu_sections, ALL_MENU_SECTIONS, MENU_DEFAULT_ON);
        if !ARTISTS_VIEWS.contains(&self.artists_view.as_str()) {
            self.artists_view = ARTISTS_VIEWS[0].into();
        }
        self.volume = self.volume.clamp(0.0, 1.0);
    }
}

pub fn app_dir(kind: DirKind) -> PathBuf {
    let base = match kind {
        DirKind::Config => dirs::config_dir(),
        DirKind::Cache => dirs::cache_dir(),
    }
    .unwrap_or_else(std::env::temp_dir);
    let p = base.join("Tunes");
    let _ = std::fs::create_dir_all(&p);
    p
}

pub enum DirKind {
    Config,
    Cache,
}

pub struct ConfigStore {
    path: PathBuf,
    inner: RwLock<Config>,
}

impl ConfigStore {
    pub fn load() -> Self {
        let path = app_dir(DirKind::Config).join("config.json");
        let mut cfg: Config = std::fs::read(&path)
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default();
        cfg.normalise();
        Self { path, inner: RwLock::new(cfg) }
    }

    pub fn get(&self) -> Config {
        self.inner.read().clone()
    }

    pub fn update(&self, f: impl FnOnce(&mut Config)) -> Config {
        let snapshot = {
            let mut c = self.inner.write();
            f(&mut c);
            c.normalise();
            c.clone()
        };
        let tmp = self.path.with_extension("json.tmp");
        if let Ok(bytes) = serde_json::to_vec_pretty(&snapshot) {
            if std::fs::write(&tmp, bytes).is_ok() {
                let _ = std::fs::rename(&tmp, &self.path);
            }
        }
        snapshot
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn old_config_gains_menu_sections_with_defaults() {
        // A config saved before menu sections existed.
        let mut c: Config = serde_json::from_str(r#"{"homeSections":[{"id":"random","enabled":true}]}"#).unwrap();
        c.menu_sections.clear();
        c.normalise();
        let on: Vec<_> = c.menu_sections.iter().filter(|s| s.enabled).map(|s| s.id.as_str()).collect();
        assert_eq!(on, ["artists", "albums", "playlists", "favourites"]);
        assert_eq!(c.home_sections[0].id, "random");
        assert!(c.home_sections[1..].iter().all(|s| !s.enabled));
    }

    #[test]
    fn menu_sections_keep_saved_order_and_drop_unknown() {
        let mut c = Config::default();
        c.menu_sections = vec![
            HomeSection { id: "playlists".into(), enabled: false },
            HomeSection { id: "bogus".into(), enabled: true },
            HomeSection { id: "artists".into(), enabled: true },
            HomeSection { id: "playlists".into(), enabled: true },
        ];
        c.normalise();
        let ids: Vec<_> = c.menu_sections.iter().map(|s| s.id.as_str()).collect();
        assert_eq!(ids, ["playlists", "artists", "albums", "genres", "favourites"]);
        assert!(!c.menu_sections[0].enabled);
    }

    #[test]
    fn unknown_artists_view_falls_back_to_album_artists() {
        let mut c = Config { artists_view: "composer".into(), ..Config::default() };
        c.normalise();
        assert_eq!(c.artists_view, "albumArtist");
        let mut c = Config { artists_view: "both".into(), ..Config::default() };
        c.normalise();
        assert_eq!(c.artists_view, "both");
    }
}
