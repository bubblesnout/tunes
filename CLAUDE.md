# Tunes: notes for Claude

Gamepad-first, full-screen Navidrome player styled after Steam Big Picture. The primary device is the owner's **ROG Xbox Ally X**, which boots into Steam Big Picture and launches apps with a controller. The README covers features, controls and architecture; this file records what the README doesn't: decisions, gotchas and open work.

## Requirements (from the owner)

- Fully usable with a gamepad (XInput is fine). Keyboard and mouse must work too, but it's clearly a button UI. Every binding is remappable.
- Windows first. Keep it easy to ship a Linux AppImage later.
- Music comes **only** from Navidrome via the Subsonic/OpenSubsonic API. No local folders, other services or downloads.
- Look and feel should match Steam Big Picture: palette, focus animations, "eye candy", the slide-out left menu. Themes may come later, so styles go through tokens in `src/themes/`.
- Launches fast and never signs you out (the password lives in the OS keyring; Subsonic tokens don't expire).
- Light on resources, since it mostly runs in the background.
- **Steam overlay support is a hard requirement.** See the open work below.

## Stack and key decisions

- Tauri 2 with a Svelte 5 frontend (runes) and a Rust backend. Rust owns playback, input and the server connection; the webview only presents.
- Audio is pure Rust: progressive HTTP stream → symphonia → rubato → cpal. It's gapless. Codecs symphonia can't decode (opus, wma…) fall back to server-side MP3 (`format=mp3`). There's no libmpv, by choice: no DLL to ship.
- Gamepad uses `gilrs` with the **`xinput` feature on Windows**. The default backend, Windows.Gaming.Input, detects pads but delivers no input to Win32 desktop apps. That was the first gamepad bug.
- Gamepad input only counts while Tunes is the foreground app. Focus is checked with `GetForegroundWindow` + process id (`gamepad.rs`), because Tauri's Focused events misfire when focus moves into the WebView2 child window.
- Standard XInput doesn't report the Xbox/Guide button, which belongs to Steam anyway. Never bind it.
- Bindings are stored with `_v` (`BINDINGS_VERSION` in `src/lib/input.ts`). Bump it when the default layout changes incompatibly.
- Defaults follow Steam's own mapping: A select, B back (at the root of a section B **opens the main menu**, as in Steam), ☰ Options, ⧉ View opens the main menu, X Now Playing, Y play/pause, LB/RB prev/next, LT/RT volume.
- Volume is in-app (the cpal callback gain), never the system volume.
- Album art goes through the `cover://` scheme with a disk cache (`covers.rs`), so auth tokens never reach the webview.
- Spatial navigation lives in `src/lib/nav.ts`. `data-nav` marks focusable elements, `data-nav-group` remembers the last focus in a group, `data-nav-entry` sets a group's entry point (an album's Play button), `data-nav-default` sets the initial focus, and `data-nav-scope` marks modal scopes. Elements under `[inert]` are skipped.
- Steam colours: the green Play button (`--play`) is for starting; blue (`--accent`) is only for "Resume".

## Fonts (licensing)

Motiva Sans (Steam's font) is free for personal use but **not redistributable**. The `.ttf` files are git-ignored in `public/fonts/` (see the README there). Locally, unzip `motiva-sans.zip` into `public/fonts/` using the names it lists. CI downloads them from https://font.download/dl/font/motiva-sans.zip. The app falls back to Segoe UI without them. Never commit the font files.

## Steam integration

- **Add to Steam** (Settings → Steam, `src-tauri/src/steam.rs`) parses and rewrites the binary `shortcuts.vdf`. It preserves every other entry and writes a `.bak` first. It targets the most recent account from `loginusers.vdf` and copies the artwork from `art/steam/` into `userdata/<id>/config/grid/`. Steam only reads shortcuts at startup, hence the **Restart Steam** button. Tests cover this against a fake Steam folder. **Confirmed working on the Ally.**
- The artwork comes from `node scripts/steam-art.mjs` (resvg, using Motiva Sans from `public/fonts`). The PNGs in `art/steam/` are committed and embedded with `include_bytes!`. Keep the mark in `scripts/steam-art.mjs` in sync with `src-tauri/icons/source.svg`, then run `npx tauri icon src-tauri/icons/source.svg -o src-tauri/icons` and delete the android/ios folders it creates.
- Steam sets `SteamGameId` / `SteamOverlayGameId` / `SteamClientLaunch` for anything it launches (`steam::launched_by_steam()`).

## Open work: the Steam overlay (hard requirement)

Symptom: pressing the Xbox button in Tunes plays the overlay sound, but no overlay appears.

What we've learned:
1. **v0.1.1:** WebView2 switches `--in-process-gpu --disable-direct-composition` plus a 1px keep-alive animation. Applied correctly, but **didn't help**. Since removed.
2. **v0.1.2:** diagnostics (`overlay_diag.rs`) showed `gameoverlayrenderer64.dll` **is** loaded in `tunes.exe` **and** in the WebView2 browser process (the one with d3d11). So Steam gets in. Theory: the overlay is tied to `tunes.exe`, which never presents a frame itself.
3. **v0.1.3 (awaiting a test on the Ally):** `overlay_host.rs` creates a borderless, click-through window (`WS_EX_LAYERED|TRANSPARENT|NOACTIVATE|TOOLWINDOW`, colour key black) owned by and covering the main window. It presents D3D11 frames from `tunes.exe` (blt-model `SEQUENTIAL` swap chain) while Tunes is in front: about 5 fps at rest, every vsync while Steam is drawing. Drawing is detected by sampling an 8×8 pixel grid from the presented buffer. Verified locally (no Steam on the dev PC): invisible, click-through, detection works (debug builds only: `TUNES_FAKE_OVERLAY=1` paints grey), about 3% of one core at rest at 1440p.

How to read a v0.1.3 test log (`%LOCALAPPDATA%\Tunes\tunes.log`): `Steam overlay host ready` means the host started. `Steam overlay drawing: full frame rate` means Steam drew into the host. If it never appears, Steam isn't hooking the host's presents. Things to try then: a DirectComposition / flip-model swap chain, or presenting on the main window's own HWND. If it works but looks wrong: black is the colour key, so Steam's darkening over the app shows as transparent. Consider a near-black key such as RGB(1,0,1), or tint the host while the overlay is open.

## Building, testing, releasing

- `npm run dev` gives a UI-only preview in a browser with **mock data** (`src/lib/mock.ts`). If the browser pane is hidden it pauses CSS animations. For screenshots, inject `*{animation-duration:0s!important;transition-duration:0s!important}`.
- `npm run check` type-checks the Svelte/TS. `cd src-tauri && cargo test --lib` runs the Rust tests.
- `npx tauri build --debug --no-bundle` builds the debug exe at `src-tauri/target/debug/tunes.exe`.
- To simulate a Steam launch locally, set `SteamClientLaunch=1` before starting the exe.
- **Releases:** bump the version in `src-tauri/tauri.conf.json`, `package.json` and `src-tauri/Cargo.toml` (then `cargo update -p tunes --offline` and `npm install --package-lock-only`), plus the version string in `src/lib/screens/Settings.svelte` (General tab). Commit, then `git tag -a vX.Y.Z -m "Tunes X.Y.Z"` and push the tag. `.github/workflows/release.yml` builds on windows-latest and publishes a GitHub Release with the NSIS installer and a portable zip (about 10 minutes). Builds are unsigned (Azure Artifact Signing is a possible future step).
- Logs: `%LOCALAPPDATA%\Tunes\tunes.log` (Windows), truncated on each launch.

## Git and GitHub

- Repo: https://github.com/bubblesnout/tunes (public). Push as the **`bubblesnout`** GitHub account.
- Commits in this repo use the repo-local email `6344088+bubblesnout@users.noreply.github.com`, so personal or work email never appears publicly. On a fresh clone, set it: `git config user.email 6344088+bubblesnout@users.noreply.github.com` and `git config user.name "Dean French"`.
- If the GitHub CLI has several accounts, push with `gh auth switch --user bubblesnout`, then `git -c credential.helper= -c credential.helper='!gh auth git-credential' push ...`.
- Commit or push only when the owner asks. They usually ask for releases as tagged versions.
