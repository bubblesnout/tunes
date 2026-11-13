# Tunes

A gamepad-first, full-screen music player for **Navidrome** (any Subsonic/OpenSubsonic server), styled after Steam Big Picture mode. Built for handhelds like the ROG Xbox Ally X running Big Picture as the shell, but fully usable with keyboard and mouse.

## Features (v0.1)

- Sign in once. Your password is stored in the OS credential store (Windows Credential Manager / Secret Service), and Subsonic tokens never expire, so you stay signed in indefinitely.
- Fast start: Home renders instantly from a disk cache while it refreshes in the background, and album art is cached on disk.
- Home sections that you can reorder and toggle: Continue Listening, Recently Added, Recently Played, Most Played, Favourites, Discover (random), Top Rated, Playlists.
- Main-menu library pages under Home: Artists, Albums, Playlists, Genres and Favourites. Toggle and reorder them in Settings → Main Menu, where you also choose whether Artists lists album artists, track artists (including featured artists), or both as separate pages.
- A–Z fast scrolling on those pages, as in Big Picture's All Games list: hold up or down and the screen dims behind a large letter. Keep holding and it moves a letter at a time.
- Album, playlist and artist pages: play, shuffle, play next, add to queue.
- Now Playing: cover art, progress, shuffle, previous, play/pause, next, repeat, and Up Next.
- Gapless playback, decoded locally (FLAC, MP3, AAC/M4A, ALAC, Vorbis, WAV). Formats it can't decode, such as Opus, fall back to server-side MP3 transcoding.
- In-app volume that only affects Tunes.
- Steam overlay support when launched from Steam (Xbox/Guide button).
- Integrates with Windows media controls: media keys, the Windows/Game Bar media overlay, and the Ally's quick settings.
- Scrobbles plays to Navidrome, which is how Recently Played and Most Played stay current.
- The queue and position are restored on the next launch.
- Every control can be remapped, for both gamepad and keyboard.

## Default controls

| Action | Gamepad | Keyboard |
|---|---|---|
| Navigate | D-pad / left stick | Arrow keys |
| Fast scroll A–Z (library pages) | Hold D-pad ↑/↓ | Hold ↑/↓ |
| Select | A | Enter |
| Back | B | Esc / Backspace |
| Main menu (slide-out) | View ⧉, or push left from the left edge | M |
| Options (play next, queue…) | Menu ☰ | O |
| Now Playing | X (or the top-right mini player) | N |
| Play / pause | Y | Space |
| Previous / next track | LB / RB | `[` / `]` |
| Volume down / up (in-app) | LT / RT (hold to repeat) | `-` / `=` |
| Page scroll | Right stick ↑/↓ | PgUp / PgDn |
| Toggle fullscreen | — | F11 |

Gamepad input is ignored while Tunes isn't the focused window, so navigating Steam doesn't also control Tunes. The Guide/Xbox button is never bound because it belongs to Steam.

## Using it from Steam Big Picture

1. In Tunes, open **Settings → Steam → Add to Steam**. This creates the shortcut with its library artwork (capsules, hero, logo, icon) for your most recent Steam account, then offers to restart Steam so it shows up. (Or add `Tunes.exe` manually with **Add a Non-Steam Game**.)
2. In the shortcut's **Controller** settings, choose the **Gamepad** template, or disable Steam Input for it. Otherwise Steam may translate the controller into keyboard and mouse input.
3. Press Steam + X to open Steam's on-screen keyboard on the sign-in screen.

## Releases

Pushing a version tag (e.g. `v0.2.0`) runs the [Release workflow](.github/workflows/release.yml) on GitHub Actions. It downloads Motiva Sans, type-checks, runs the Rust tests, builds Tunes for Windows and publishes a GitHub Release with:

- `Tunes_<version>_x64-setup.exe`: the installer
- `Tunes_<version>_x64-portable.zip`: just `tunes.exe` (needs the WebView2 runtime, which Windows 11 already includes)

The app's version comes from the tag. The builds aren't code-signed yet, so Windows SmartScreen may warn on first run.

```bash
git tag v0.2.0
git push origin v0.2.0
```

## Development

Requires Rust (stable) and Node 20+.

**Font:** the UI uses Motiva Sans, which isn't included in this repo for licensing reasons. Put the `.ttf` files in `public/fonts/` (see [public/fonts/README.md](public/fonts/README.md)). Without them, Tunes falls back to Segoe UI.

```bash
npm install
npm run tauri dev               # run with hot reload
npm run tauri build             # release build + NSIS installer
npm run check                   # type-check the frontend
npm run dev                     # UI-only preview in a browser, with mock data
node scripts/steam-art.mjs      # regenerate the Steam artwork in art/steam
```

Linux (AppImage) builds need the usual Tauri prerequisites (`webkit2gtk-4.1`, `libayatana-appindicator3`), plus `libasound2-dev`, `libudev-dev` and `libdbus-1-dev`. Then run `npm run tauri build`.

## Architecture

```
src-tauri/src
  lib.rs          Tauri setup + commands
  subsonic.rs     Subsonic API client (token auth)
  credentials.rs  OS keyring storage
  config.rs       settings (JSON in the platform config dir)
  player.rs       queue/transport, scrobbling, session restore
  audio/          stream.rs (progressive HTTP), decoder.rs (symphonia + rubato, gapless), output.rs (cpal + volume)
  gamepad.rs      gilrs → raw "pad" events (focus-gated, with repeat)
  media.rs        SMTC / MPRIS via souvlaki
  covers.rs       cover:// protocol with disk cache
  overlay_host.rs invisible D3D11 window that gives the Steam overlay frames to draw on
src
  lib/input.ts    action bindings (remappable), keyboard + pad dispatch
  lib/nav.ts      spatial focus navigation
  lib/library.svelte.ts     library lists for the menu pages (sorting, A–Z letters, session cache)
  lib/fastscroll.svelte.ts  hold-to-scroll with the big-letter overlay
  lib/screens     Login, Home, Collection (album/playlist), Artist, Library (A–Z grids), Favourites, NowPlaying, Settings
  themes/         theme tokens (bigpicture.css), so new themes are just new token sets
```

The Rust side owns playback, input and the server connection. The webview only presents, so music keeps playing regardless of what the UI is doing.
