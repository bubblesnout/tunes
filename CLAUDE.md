# Tunes: notes for Claude

Gamepad-first, full-screen Navidrome player styled after Steam Big Picture. The primary device is the owner's **ROG Xbox Ally X**, which boots into Steam Big Picture and launches apps with a controller. The README covers features, controls and architecture; this file records what the README doesn't: decisions, gotchas and open work.

## Requirements (from the owner)

- Fully usable with a gamepad (XInput is fine). Keyboard and mouse must work too, but it's clearly a button UI. Every binding is remappable.
- Windows first. Keep it easy to ship a Linux AppImage later.
- Music comes **only** from Navidrome via the Subsonic/OpenSubsonic API. No local folders, other services or downloads.
- Look and feel should match Steam Big Picture: palette, focus animations, "eye candy", the slide-out left menu. Themes may come later, so styles go through tokens in `src/themes/`.
- Launches fast and never signs you out (the password lives in the OS keyring; Subsonic tokens don't expire).
- Light on resources, since it mostly runs in the background.
- **Steam overlay support is a hard requirement.** Solved; see below.

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
- **Library pages** (Artists, Albums, Playlists, Genres, Favourites) sit under Home in the main menu; Now Playing is at the top. Which appear, and their order, is `menuSections` in config (Settings → Main Menu). They're roots like Home: `navigate` resets the stack, so B opens the menu.
- **Album artists vs track artists** (`artistsView`: `albumArtist` | `artist` | `both`). Verified against Navidrome's source (not a live server): `getArtists` returns **album artists only** (`GetIndex(..., model.RoleAlbumArtist)`, not configurable). `search3` with the literal query `""` returns every artist with OpenSubsonic `roles`, so track artists = role `artist` (everyone if the server sends no roles). `getArtist` lists only album-artist albums unless the server's `Subsonic.ArtistParticipations` is on (default off), so the Artist page adds **Appears On** from `search3` on the artist's name, keeping songs whose `artists[]`/`artistId` match. "Both" splits the menu's Artists entry into Album Artists + Artists (`menuPages()` in `app.svelte.ts`).
- Lists load once per session into `library.svelte.ts` and refresh in the background on revisit. Albums come from `getAlbumList2` paged 500 at a time. Artists keep the server's own index letters; albums, playlists and genres are sorted client-side with the server's `ignoredArticles` ("The Black…" files under B).
- **A–Z fast scroll** (`fastscroll.svelte.ts`): tiles carry `data-letter`. A held up/down (`repeat` events) on a lettered tile shows `LetterOverlay`, scrolls instantly, and after 8 repeats (~1 s on a pad) jumps a letter per repeat. Modelled on Big Picture's All Games list from memory, not side by side; tune `LETTER_JUMP_AFTER` if it feels off.
- Grids use `content-visibility: auto` and covers load lazily, so libraries with thousands of albums stay light. Library pages remember the last focused tile, so Back returns to it.
- Footer button glyphs (`Glyph.svelte`) are SVG for pad buttons, so they centre regardless of font metrics. Never go back to Unicode ☰/⧉ characters; text keys use `text-box: trim-both cap alphabetic` to centre.
- Steam colours: the green Play button (`--play`) is for starting; blue (`--accent`) is only for "Resume".

## Fonts (licensing)

Motiva Sans (Steam's font) is free for personal use but **not redistributable**. The `.ttf` files are git-ignored in `public/fonts/` (see the README there). Locally, unzip `motiva-sans.zip` into `public/fonts/` using the names it lists. CI downloads them from https://font.download/dl/font/motiva-sans.zip. The app falls back to Segoe UI without them. Never commit the font files.

## Steam integration

- **Add to Steam** (Settings → Steam, `src-tauri/src/steam.rs`) parses and rewrites the binary `shortcuts.vdf`. It preserves every other entry and writes a `.bak` first. It targets the most recent account from `loginusers.vdf` and copies the artwork from `art/steam/` into `userdata/<id>/config/grid/`. Steam only reads shortcuts at startup, hence the **Restart Steam** button. Tests cover this against a fake Steam folder. **Confirmed working on the Ally.**
- The artwork comes from `node scripts/steam-art.mjs` (resvg, using Motiva Sans from `public/fonts`). The PNGs in `art/steam/` are committed and embedded with `include_bytes!`. Keep the mark in `scripts/steam-art.mjs` in sync with `src-tauri/icons/source.svg`, then run `npx tauri icon src-tauri/icons/source.svg -o src-tauri/icons` and delete the android/ios folders it creates.
- Steam sets `SteamGameId` / `SteamOverlayGameId` / `SteamClientLaunch` for anything it launches (`steam::launched_by_steam()`).

## The Steam overlay (hard requirement, solved in v0.1.3)

Steam injects its overlay into `tunes.exe` and ties it to that process, but all drawing happens in WebView2's own processes, so originally the Xbox button played the overlay sound and nothing appeared. **Confirmed working on the Ally with v0.1.3.**

The fix is `overlay_host.rs`: a borderless, click-through window (`WS_EX_LAYERED|TRANSPARENT|NOACTIVATE|TOOLWINDOW`, colour key black) owned by and covering the main window. It presents D3D11 frames from `tunes.exe` (blt-model `SEQUENTIAL` swap chain) while Tunes is in front: about 5 fps at rest, every vsync while Steam is drawing (detected by sampling an 8×8 pixel grid from the presented buffer). About 3% of one core at rest at 1440p. It runs only when launched by Steam and the Settings → Steam toggle is on (`steam::overlay_compat_active`). Debug builds: `TUNES_FAKE_OVERLAY=1` paints grey to stand in for Steam.

Log lines (`%LOCALAPPDATA%\Tunes	unes.log`): `Steam overlay host ready` and `Steam overlay drawing: full frame rate` / `idle: low frame rate`.

Dead ends, so they aren't retried: WebView2 switches (`--in-process-gpu --disable-direct-composition`) plus a keep-alive animation did nothing (v0.1.1). Module diagnostics (v0.1.2, since removed) showed `gameoverlayrenderer64.dll` loads in both `tunes.exe` and the WebView2 browser process.

Known cosmetic caveat: black is the colour key, so any pure-black pixels Steam draws (e.g. its dimming) show through as transparent. If that ever looks wrong, use a near-black key such as RGB(1,0,1).

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
