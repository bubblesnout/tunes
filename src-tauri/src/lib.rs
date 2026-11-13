mod audio;
mod config;
mod covers;
mod credentials;
mod gamepad;
mod logging;
mod media;
mod overlay_diag;
mod overlay_host;
mod player;
mod steam;
mod subsonic;

use config::{app_dir, Config, ConfigStore, DirKind, HomeSection};
use parking_lot::RwLock;
use player::{Player, Repeat, SharedClient, Snapshot};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use subsonic::{Album, AlbumDetail, Client, Credentials, Playlist, PlaylistDetail, Song};
use tauri::{Manager, State, WindowEvent};

pub struct AppState {
    pub config: Arc<ConfigStore>,
    pub client: SharedClient,
    pub player: Arc<Player>,
}

type CmdResult<T> = Result<T, String>;

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

fn client(state: &AppState) -> CmdResult<Client> {
    state.client.read().clone().ok_or_else(|| "Not signed in".to_string())
}

// ---------- session / auth -------------------------------------------------------------------

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Bootstrap {
    signed_in: bool,
    config: Config,
    player: Snapshot,
}

#[tauri::command]
fn bootstrap(state: State<AppState>) -> Bootstrap {
    Bootstrap {
        signed_in: state.client.read().is_some(),
        config: state.config.get(),
        player: state.player.snapshot(),
    }
}

#[tauri::command]
async fn login(state: State<'_, AppState>, server: String, username: String, password: String) -> CmdResult<Config> {
    let c = Client::new(Credentials { server, username: username.trim().to_string(), password });
    c.ping().await.map_err(err)?;
    let creds = c.creds().clone();
    credentials::save(&creds.server, &creds.username, &creds.password)
        .map_err(|e| format!("Signed in, but couldn't store credentials securely: {e}"))?;
    *state.client.write() = Some(c);
    Ok(state.config.update(|cfg| {
        cfg.server_url = creds.server;
        cfg.username = creds.username;
    }))
}

#[tauri::command]
fn logout(state: State<AppState>) -> Config {
    let cfg = state.config.get();
    credentials::delete(&cfg.server_url, &cfg.username);
    *state.client.write() = None;
    state.player.pause();
    let _ = std::fs::remove_file(home_cache_path());
    state.config.update(|c| c.username.clear())
}

/// Verify stored credentials in the background without blocking startup.
#[tauri::command]
async fn check_connection(state: State<'_, AppState>) -> CmdResult<()> {
    client(&state)?.ping().await.map_err(err)
}

// ---------- library ------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Default, Clone)]
#[serde(rename_all = "camelCase")]
struct HomeData {
    albums: HashMap<String, Vec<Album>>,
    playlists: Vec<Playlist>,
}

fn home_cache_path() -> std::path::PathBuf {
    app_dir(DirKind::Cache).join("home.json")
}

/// Cached Home data, returned instantly at startup (may be empty).
#[tauri::command]
fn get_home_cached() -> HomeData {
    std::fs::read(home_cache_path())
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}

#[tauri::command]
async fn get_home(state: State<'_, AppState>) -> CmdResult<HomeData> {
    let c = client(&state)?;
    let enabled: Vec<String> = state.config.get().home_sections.into_iter().filter(|s| s.enabled).map(|s| s.id).collect();
    let kinds: Vec<(&str, &str)> = [
        ("recentlyAdded", "newest"),
        ("recentlyPlayed", "recent"),
        ("mostPlayed", "frequent"),
        ("starred", "starred"),
        ("random", "random"),
        ("highestRated", "highest"),
    ]
    .into_iter()
    .filter(|(id, _)| enabled.iter().any(|e| e == id))
    .collect();

    let futures = kinds.iter().map(|(id, kind)| {
        let c = c.clone();
        async move { (id.to_string(), c.album_list(kind, 24).await) }
    });
    let playlists_fut = async {
        if enabled.iter().any(|e| e == "playlists") {
            c.playlists().await.unwrap_or_default()
        } else {
            Vec::new()
        }
    };
    let (results, playlists) = tokio::join!(futures::future::join_all(futures), playlists_fut);
    let mut data = HomeData { playlists, ..Default::default() };
    let mut first_err = None;
    for (id, r) in results {
        match r {
            Ok(list) => {
                data.albums.insert(id, list);
            }
            Err(e) => {
                first_err.get_or_insert(e.to_string());
            }
        }
    }
    if data.albums.is_empty() {
        if let Some(e) = first_err {
            return Err(e);
        }
    }
    if let Ok(b) = serde_json::to_vec(&data) {
        let _ = std::fs::write(home_cache_path(), b);
    }
    Ok(data)
}

#[tauri::command]
async fn get_album(state: State<'_, AppState>, id: String) -> CmdResult<AlbumDetail> {
    client(&state)?.album(&id).await.map_err(err)
}

#[tauri::command]
async fn get_playlist(state: State<'_, AppState>, id: String) -> CmdResult<PlaylistDetail> {
    client(&state)?.playlist(&id).await.map_err(err)
}

// ---------- player -------------------------------------------------------------------------

#[tauri::command]
fn player_state(state: State<AppState>) -> Snapshot {
    state.player.snapshot()
}

#[tauri::command]
fn player_play_songs(state: State<AppState>, songs: Vec<Song>, index: usize) {
    state.player.play_queue(songs, index);
}

#[tauri::command]
fn player_enqueue(state: State<AppState>, songs: Vec<Song>, next: bool) {
    state.player.enqueue(songs, next);
}

#[tauri::command]
fn player_command(state: State<AppState>, cmd: String, value: Option<f64>) -> Snapshot {
    let p = &state.player;
    match cmd.as_str() {
        "toggle" => p.toggle(),
        "play" => p.play(),
        "pause" => p.pause(),
        "next" => p.next(),
        "prev" => p.prev(),
        "seek" => p.seek(value.unwrap_or(0.0)),
        "seekBy" => p.seek_relative(value.unwrap_or(0.0)),
        "jump" => p.jump(value.unwrap_or(0.0) as usize),
        "volume" => {
            p.set_volume(value.unwrap_or(0.8) as f32);
        }
        "volumeBy" => {
            p.change_volume(value.unwrap_or(0.0) as f32);
        }
        "repeat" => p.set_repeat(match value.unwrap_or(0.0) as i32 {
            1 => Repeat::All,
            2 => Repeat::One,
            _ => Repeat::Off,
        }),
        "shuffle" => p.toggle_shuffle(),
        _ => {}
    }
    p.snapshot()
}

// ---------- settings -----------------------------------------------------------------------

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SettingsPatch {
    home_sections: Option<Vec<HomeSection>>,
    bindings: Option<serde_json::Value>,
    theme: Option<String>,
    fullscreen: Option<bool>,
    steam_overlay: Option<bool>,
}

#[tauri::command]
fn save_settings(state: State<AppState>, patch: SettingsPatch) -> Config {
    state.config.update(|c| {
        if let Some(h) = patch.home_sections {
            c.home_sections = h;
        }
        if let Some(b) = patch.bindings {
            c.bindings = b;
        }
        if let Some(t) = patch.theme {
            c.theme = t;
        }
        if let Some(f) = patch.fullscreen {
            c.fullscreen = f;
        }
        if let Some(o) = patch.steam_overlay {
            c.steam_overlay = o;
        }
    })
}

#[tauri::command]
async fn steam_status() -> steam::SteamStatus {
    tauri::async_runtime::spawn_blocking(steam::status).await.unwrap_or(steam::SteamStatus {
        installed: false, running: false, added: false, account: None,
    })
}

#[tauri::command]
async fn steam_shortcut(add: bool) -> CmdResult<steam::SteamStatus> {
    tauri::async_runtime::spawn_blocking(move || if add { steam::add() } else { steam::remove() })
        .await
        .map_err(err)?
        .map_err(err)
}

#[tauri::command]
async fn steam_restart() -> CmdResult<()> {
    tauri::async_runtime::spawn_blocking(steam::restart).await.map_err(err)?.map_err(err)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct LaunchInfo {
    launched_by_steam: bool,
    overlay_compat: bool,
}

#[tauri::command]
fn launch_info(state: State<AppState>) -> LaunchInfo {
    LaunchInfo {
        launched_by_steam: steam::launched_by_steam(),
        overlay_compat: steam::overlay_compat_active(state.config.get().steam_overlay),
    }
}

#[tauri::command]
fn gamepad_status() -> gamepad::PadStatus {
    gamepad::status()
}

#[tauri::command]
fn quit(app: tauri::AppHandle, state: State<AppState>) {
    state.player.save_session();
    app.exit(0);
}

// ---------- entry --------------------------------------------------------------------------

pub fn run() {
    logging::init();
    let config = Arc::new(ConfigStore::load());
    let focused = Arc::new(AtomicBool::new(true));
    let focused_ev = focused.clone();

    tauri::Builder::default()
        .register_asynchronous_uri_scheme_protocol("cover", covers::handle)
        .setup(move |app| {
            let cfg = config.get();
            // Re-sign-in silently from the OS credential store: instant, no network needed.
            let client: SharedClient = Arc::new(RwLock::new(None));
            if !cfg.server_url.is_empty() && !cfg.username.is_empty() {
                if let Some(pw) = credentials::load(&cfg.server_url, &cfg.username) {
                    *client.write() = Some(Client::new(Credentials {
                        server: cfg.server_url.clone(),
                        username: cfg.username.clone(),
                        password: pw,
                    }));
                }
            }
            let cfg_store = config.clone();
            let player = Player::new(app.handle().clone(), client.clone(), cfg.volume, move |v| {
                cfg_store.update(|c| c.volume = v);
            });

            let overlay = steam::overlay_compat_active(cfg.steam_overlay);
            let builder = tauri::WebviewWindowBuilder::new(app, "main", tauri::WebviewUrl::default())
                .title("Tunes")
                .inner_size(1280.0, 800.0)
                .min_inner_size(960.0, 540.0)
                .fullscreen(cfg.fullscreen)
                .decorations(false)
                .background_color(tauri::window::Color(14, 20, 27, 255));
            let window = builder.build()?;
            if steam::launched_by_steam() {
                // Snapshot once WebView2 has started its processes and drawn.
                // (XInput's standard API doesn't report the Xbox button, so also log on a timer:
                // open the overlay before the 45s snapshot to capture it.)
                std::thread::spawn(|| {
                    std::thread::sleep(std::time::Duration::from_secs(12));
                    overlay_diag::log("12s after launch");
                    std::thread::sleep(std::time::Duration::from_secs(33));
                    overlay_diag::log("45s after launch");
                });
            }
            #[cfg(windows)]
            let hwnd = window.hwnd().ok().map(|h| h.0 as isize);
            #[cfg(not(windows))]
            let hwnd = None;
            media::start(&player, hwnd);
            if overlay {
                // Give Steam's overlay (injected into tunes.exe) frames of our own to draw on.
                log::info!("launched by Steam: starting Steam overlay host");
                if let Some(h) = hwnd {
                    overlay_host::start(h);
                }
            }
            gamepad::start(app.handle().clone(), focused.clone());

            app.manage(AppState { config: config.clone(), client, player });
            Ok(())
        })
        .on_window_event(move |window, event| match event {
            WindowEvent::Focused(f) => focused_ev.store(*f, std::sync::atomic::Ordering::Relaxed),
            WindowEvent::CloseRequested { .. } | WindowEvent::Destroyed => {
                if let Some(s) = window.try_state::<AppState>() {
                    s.player.save_session();
                }
            }
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![
            bootstrap,
            login,
            logout,
            check_connection,
            get_home_cached,
            get_home,
            get_album,
            get_playlist,
            player_state,
            player_play_songs,
            player_enqueue,
            player_command,
            save_settings,
            gamepad_status,
            steam_status,
            launch_info,
            steam_shortcut,
            steam_restart,
            quit,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Tunes");
}
