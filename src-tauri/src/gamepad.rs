//! Gamepad input via gilrs (XInput on Windows, evdev on Linux).
//! Emits raw, named button events to the UI, which maps them to actions using the
//! user's bindings. Input is only forwarded while the window is focused so that
//! navigating Steam Big Picture doesn't also drive Tunes.

use gilrs::{Axis, Button, EventType, Gilrs};
use serde::Serialize;
use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
#[cfg(not(windows))]
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};

const REPEAT_DELAY: Duration = Duration::from_millis(380);
const REPEAT_RATE: Duration = Duration::from_millis(85);
const STICK_ON: f32 = 0.6;
const STICK_OFF: f32 = 0.35;

/// Connected controllers + last raw input, for the diagnostics shown in Settings.
#[derive(Serialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct PadStatus {
    pub backend: &'static str,
    pub pads: Vec<String>,
    pub focused: bool,
    pub last_input: Option<String>,
}

static STATUS: parking_lot::Mutex<Option<PadStatus>> = parking_lot::Mutex::new(None);

pub fn status() -> PadStatus {
    STATUS.lock().clone().unwrap_or_default()
}

fn update_status(app: &AppHandle, f: impl FnOnce(&mut PadStatus)) {
    let snapshot = {
        let mut g = STATUS.lock();
        let s = g.get_or_insert_with(|| PadStatus { backend: BACKEND, ..Default::default() });
        f(s);
        s.clone()
    };
    let _ = app.emit("pad-status", snapshot);
}

fn connected_pads(g: &Gilrs) -> Vec<String> {
    g.gamepads().filter(|(_, p)| p.is_connected()).map(|(_, p)| p.name().to_string()).collect()
}

#[cfg(windows)]
const BACKEND: &str = "XInput";
#[cfg(not(windows))]
const BACKEND: &str = "evdev";

#[derive(Serialize, Clone)]
struct PadEvent {
    button: &'static str,
    pressed: bool,
    repeat: bool,
}

/// Buttons that auto-repeat when held (navigation, volume, seeking).
fn repeats(name: &str) -> bool {
    name.starts_with("DPad") || name.contains("Stick") || name == "LT" || name == "RT" || name == "LB" || name == "RB"
}

fn button_name(b: Button) -> Option<&'static str> {
    Some(match b {
        Button::South => "A",
        Button::East => "B",
        Button::West => "X",
        Button::North => "Y",
        Button::LeftTrigger => "LB",
        Button::RightTrigger => "RB",
        Button::LeftTrigger2 => "LT",
        Button::RightTrigger2 => "RT",
        Button::Select => "View",
        Button::Start => "Menu",
        Button::LeftThumb => "LS",
        Button::RightThumb => "RS",
        Button::DPadUp => "DPadUp",
        Button::DPadDown => "DPadDown",
        Button::DPadLeft => "DPadLeft",
        Button::DPadRight => "DPadRight",
        // Guide/Mode belongs to Steam; never bind it.
        _ => return None,
    })
}

struct Held {
    since: Instant,
    last: Instant,
}

pub fn start(app: AppHandle, focused: Arc<AtomicBool>) {
    std::thread::Builder::new()
        .name("tunes-gamepad".into())
        .spawn(move || run(app, focused))
        .expect("spawn gamepad thread");
}

/// Is Tunes the app the user is currently using?
///
/// On Windows we ask the OS directly which process owns the foreground window. Tauri's
/// window focus events aren't reliable here: keyboard focus moves into the WebView2 child
/// window, which makes the main window report that it lost focus.
#[cfg(windows)]
fn is_focused(_fallback: &AtomicBool) -> bool {
    use windows_sys::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.is_null() {
            return false;
        }
        let mut pid = 0u32;
        GetWindowThreadProcessId(hwnd, &mut pid);
        pid == std::process::id()
    }
}

#[cfg(not(windows))]
fn is_focused(fallback: &AtomicBool) -> bool {
    fallback.load(Ordering::Relaxed)
}

fn run(app: AppHandle, focused: Arc<AtomicBool>) {
    let mut gilrs = match Gilrs::new() {
        Ok(g) => {
            let pads = connected_pads(&g);
            log::info!("gamepad backend {BACKEND}; controllers: {pads:?}");
            update_status(&app, |s| s.pads = pads);
            g
        }
        Err(e) => {
            log::error!("gamepad support unavailable: {e}");
            return;
        }
    };
    let mut held: HashMap<&'static str, Held> = HashMap::new();
    // Per stick: current virtual direction.
    let mut stick_dir: HashMap<&'static str, Option<&'static str>> = HashMap::new();
    let mut axes: HashMap<Axis, f32> = HashMap::new();
    let mut was_focused = is_focused(&focused);
    log::info!("window focus for gamepad input at start: {was_focused}");
    update_status(&app, |s| s.focused = was_focused);

    let emit = |app: &AppHandle, button: &'static str, pressed: bool, repeat: bool| {
        let _ = app.emit("pad", PadEvent { button, pressed, repeat });
    };

    loop {
        let focused_now = is_focused(&focused);
        if focused_now != was_focused {
            log::info!("window focus for gamepad input: {focused_now}");
            update_status(&app, |s| s.focused = focused_now);
        }
        if was_focused && !focused_now {
            // Release everything when we lose focus.
            for (b, _) in held.drain() {
                emit(&app, b, false, false);
            }
            stick_dir.clear();
        }
        was_focused = focused_now;

        // Poll quickly while something is held (for repeats), slowly otherwise.
        let timeout = if !held.is_empty() {
            Duration::from_millis(16)
        } else if focused_now {
            Duration::from_millis(100)
        } else {
            Duration::from_millis(250)
        };
        // Block for the first event, then drain without blocking so repeats keep ticking.
        let mut first = gilrs.next_event_blocking(Some(timeout));
        while let Some(ev) = first.take().or_else(|| gilrs.next_event()) {
            if matches!(ev.event, EventType::Connected | EventType::Disconnected) {
                let pads = connected_pads(&gilrs);
                log::info!("controllers changed: {pads:?}");
                update_status(&app, |s| s.pads = pads);
            }
            if !focused_now {
                continue;
            }
            if let EventType::ButtonPressed(b, _) = ev.event {
                let label = button_name(b).map(str::to_string).unwrap_or_else(|| format!("{b:?}"));
                update_status(&app, |s| s.last_input = Some(label));
            }
            match ev.event {
                EventType::ButtonPressed(b, _) => {
                    if let Some(name) = button_name(b) {
                        if !held.contains_key(name) {
                            let now = Instant::now();
                            held.insert(name, Held { since: now, last: now });
                            emit(&app, name, true, false);
                        }
                    }
                }
                EventType::ButtonReleased(b, _) => {
                    if let Some(name) = button_name(b) {
                        if held.remove(name).is_some() {
                            emit(&app, name, false, false);
                        }
                    }
                }
                EventType::AxisChanged(axis, value, _) => {
                    axes.insert(axis, value);
                    for (stick, xa, ya) in [
                        ("LStick", Axis::LeftStickX, Axis::LeftStickY),
                        ("RStick", Axis::RightStickX, Axis::RightStickY),
                    ] {
                        if axis != xa && axis != ya {
                            continue;
                        }
                        let x = *axes.get(&xa).unwrap_or(&0.0);
                        let y = *axes.get(&ya).unwrap_or(&0.0);
                        let cur = stick_dir.get(stick).copied().flatten();
                        let mag = x.abs().max(y.abs());
                        let new = if mag >= STICK_ON || (cur.is_some() && mag >= STICK_OFF) {
                            Some(stick_direction(stick, x, y))
                        } else {
                            None
                        };
                        if new != cur {
                            if let Some(old) = cur {
                                held.remove(old);
                                emit(&app, old, false, false);
                            }
                            if let Some(n) = new {
                                let now = Instant::now();
                                held.insert(n, Held { since: now, last: now });
                                emit(&app, n, true, false);
                            }
                            stick_dir.insert(stick, new);
                        }
                    }
                }
                EventType::Disconnected => {
                    for (b, _) in held.drain() {
                        emit(&app, b, false, false);
                    }
                    stick_dir.clear();
                    axes.clear();
                }
                _ => {}
            }
        }
        // Auto-repeat.
        let now = Instant::now();
        for (name, h) in held.iter_mut() {
            if repeats(name) && now.duration_since(h.since) >= REPEAT_DELAY && now.duration_since(h.last) >= REPEAT_RATE {
                h.last = now;
                emit(&app, name, true, true);
            }
        }
    }
}

fn stick_direction(stick: &str, x: f32, y: f32) -> &'static str {
    let horizontal = x.abs() > y.abs();
    match (stick, horizontal) {
        ("LStick", true) if x > 0.0 => "LStickRight",
        ("LStick", true) => "LStickLeft",
        ("LStick", false) if y > 0.0 => "LStickUp",
        ("LStick", false) => "LStickDown",
        (_, true) if x > 0.0 => "RStickRight",
        (_, true) => "RStickLeft",
        (_, false) if y > 0.0 => "RStickUp",
        _ => "RStickDown",
    }
}
