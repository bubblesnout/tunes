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
    /// action -> list of inputs (keyboard `Key:<code>` or gamepad `Pad:<button>`)
    pub bindings: serde_json::Value,
    pub volume: f32,
    pub theme: String,
    pub fullscreen: bool,
    /// Apply WebView2 switches that let the Steam overlay draw when launched from Steam.
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

impl Default for Config {
    fn default() -> Self {
        Self {
            server_url: String::new(),
            username: String::new(),
            home_sections: ALL_SECTIONS
                .iter()
                .map(|s| HomeSection { id: s.to_string(), enabled: DEFAULT_ON.contains(s) })
                .collect(),
            bindings: serde_json::Value::Null,
            volume: 0.8,
            theme: "bigpicture".into(),
            fullscreen: true,
            steam_overlay: true,
        }
    }
}

impl Config {
    /// Ensure every known section appears exactly once (new sections get appended disabled).
    fn normalise(&mut self) {
        self.home_sections.retain(|s| ALL_SECTIONS.contains(&s.id.as_str()));
        let mut seen = std::collections::HashSet::new();
        self.home_sections.retain(|s| seen.insert(s.id.clone()));
        for id in ALL_SECTIONS {
            if !self.home_sections.iter().any(|s| s.id == *id) {
                self.home_sections.push(HomeSection { id: id.to_string(), enabled: false });
            }
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
