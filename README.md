# Tunes

A gamepad-first, full-screen music player for **Navidrome** (any Subsonic/OpenSubsonic server), styled after Steam Big Picture mode. Built for handhelds like the ROG Xbox Ally X running Big Picture as the shell, but fully usable with keyboard and mouse.

## Features (v0.1)

- Sign in once. Your password is stored in the OS credential store (Windows Credential Manager / Secret Service), and Subsonic tokens never expire, so you stay signed in indefinitely.
- Fast start: Home renders instantly from a disk cache while it refreshes in the background, and album art is cached on disk.
- Home sections that you can reorder and toggle: Continue Listening, Recently Added, Recently Played, Most Played, Favourites, Discover (random), Top Rated, Playlists.
- Album and playlist views: play, shuffle, play next, add to queue.
- Now Playing: cover art, progress, shuffle, previous, play/pause, next, repeat, and Up Next.
- Gapless playback, decoded locally (FLAC, MP3, AAC/M4A, ALAC, Vorbis, WAV). Formats it can't decode, such as Opus, fall back to server-side MP3 transcoding.
- In-app volume that only affects Tunes.
- Integrates with Windows media controls: media keys, the Windows/Game Bar media overlay, and the Ally's quick settings.
- Scrobbles plays to Navidrome, which is how Recently Played and Most Played stay current.
- The queue and position are restored on the next launch.
- Every control can be remapped, for both gamepad and keyboard.

## Default controls

| Action | Gamepad | Keyboard |
|---|---|---|
| Navigate | D-pad / left stick | Arrow keys |
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

1. In Steam, choose **Add a Non-Steam Game** and select `Tunes.exe`.
2. In the shortcut's **Controller** settings, choose the **Gamepad** template, or disable Steam Input for it. Otherwise Steam may translate the controller into keyboard and mouse input.
3. Press Steam + X to open Steam's on-screen keyboard on the sign-in screen.

## Development

Requires Rust (stable) and Node 20+.

**Font:** the UI uses Motiva Sans, which isn't included in this repo for licensing reasons. Put the `.ttf` files in `public/fonts/` (see [public/fonts/README.md](public/fonts/README.md)). Without them, Tunes falls back to Segoe UI.

```bash
npm install
npm run tauri dev               # run with hot reload
npm run tauri build             # release build + NSIS installer
npm run check                   # type-check the frontend
npm run dev                     # UI-only preview in a browser, with mock data
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
src
  lib/input.ts    action bindings (remappable), keyboard + pad dispatch
  lib/nav.ts      spatial focus navigation
  lib/screens     Login, Home, Collection (album/playlist), NowPlaying, Settings
  themes/         theme tokens (bigpicture.css), so new themes are just new token sets
```

The Rust side owns playback, input and the server connection. The webview only presents, so music keeps playing regardless of what the UI is doing.
