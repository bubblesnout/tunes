// Global reactive app state (Svelte 5 runes).

import { getCurrentWindow } from "@tauri-apps/api/window";
import { api, isTauri, on, type ArtistsView, type Config, type HomeData, type PlayerSnapshot, type Progress } from "./api";
import { setBindings, type Device } from "./input";

export type Route =
  | { name: "home" }
  | { name: "album"; id: string }
  | { name: "playlist"; id: string }
  | { name: "artist"; id: string }
  | { name: "genre"; id: string }
  | { name: LibraryPage }
  | { name: "nowPlaying" }
  | { name: "settings"; tab?: string };

/** Library pages that can appear in the main menu under Home (see Settings → Main Menu). */
export type LibraryPage = "albumArtists" | "artists" | "albums" | "playlists" | "genres" | "favourites";
export const LIBRARY_PAGES: Record<LibraryPage, { label: string; icon: string; desc: string }> = {
  albumArtists: { label: "Album Artists", icon: "artist", desc: "Every album artist, A to Z" },
  artists: { label: "Artists", icon: "artist", desc: "Every artist, A to Z" },
  albums: { label: "Albums", icon: "album", desc: "Every album, A to Z" },
  playlists: { label: "Playlists", icon: "playlist", desc: "Your playlists" },
  genres: { label: "Genres", icon: "genre", desc: "Browse albums by genre" },
  favourites: { label: "Favourites", icon: "star", desc: "Songs, albums and artists you've starred" },
};

export const ARTISTS_VIEWS: { id: ArtistsView; label: string }[] = [
  { id: "albumArtist", label: "Album artists" },
  { id: "artist", label: "Track artists" },
  { id: "both", label: "Both, separately" },
];

/**
 * Main-menu pages, in order, expanding the "artists" entry by the Artists setting:
 * album artists or track artists (both titled "Artists"), or both as two pages.
 */
type MenuPage = { id: LibraryPage; label: string; icon: string };
export function menuPages(): MenuPage[] {
  const view = app.config?.artistsView ?? "albumArtist";
  return (app.config?.menuSections ?? [])
    .filter((s) => s.enabled && s.id in LIBRARY_PAGES)
    .flatMap((s): MenuPage[] => {
      const id = s.id as LibraryPage;
      if (id !== "artists") return [{ id, ...LIBRARY_PAGES[id] }];
      const artists: MenuPage = { id: "artists", ...LIBRARY_PAGES.artists };
      const albumArtists: MenuPage = { id: "albumArtists", ...LIBRARY_PAGES.albumArtists };
      if (view === "both") return [albumArtists, artists];
      return [view === "artist" ? artists : { ...albumArtists, label: "Artists" }];
    });
}

/** Title for an artists page: "Artists" unless both kinds have their own page. */
export function artistsTitle(kind: "albumArtists" | "artists"): string {
  return kind === "albumArtists" && app.config?.artistsView === "both" ? "Album Artists" : "Artists";
}

export interface MenuItem {
  label: string;
  run: () => void | Promise<void>;
}

export const app = $state({
  ready: false,
  signedIn: false,
  config: null as Config | null,
  stack: [{ name: "home" }] as Route[],
  home: { albums: {}, playlists: [] } as HomeData,
  homeLoading: false,
  homeError: null as string | null,
  player: null as PlayerSnapshot | null,
  device: "pad" as Device,
  volumeOsd: 0, // timestamp of last volume change, drives the OSD
  toast: null as { text: string; at: number } | null,
  menu: null as { title: string; items: MenuItem[] } | null,
  drawer: false, // Steam-style slide-out main menu
  volumePopup: 0, // timestamp of last volume-slider interaction; 0 = closed
  backdrop: null as string | null, // cover id tinting the top of the screen (focused item)
  offline: false,
  launch: { launchedBySteam: false, overlayCompat: false },
});

export function route(): Route {
  return app.stack[app.stack.length - 1];
}

export function navigate(r: Route) {
  app.drawer = false;
  const top = route();
  if (JSON.stringify(top) === JSON.stringify(r)) return;
  // Top-level destinations reset the stack instead of piling up.
  // Home and the library pages are roots: B there opens the main menu, as in Steam.
  if (r.name === "home" || r.name in LIBRARY_PAGES) app.stack = [r];
  else if (r.name === "nowPlaying" || r.name === "settings") {
    app.stack = [...app.stack.filter((x) => x.name !== r.name), r];
  } else app.stack = [...app.stack, r];
}

export function back(): boolean {
  if (app.volumePopup) {
    app.volumePopup = 0;
    return true;
  }
  if (app.drawer) {
    app.drawer = false;
    return true;
  }
  if (app.menu) {
    app.menu = null;
    return true;
  }
  if (app.stack.length > 1) {
    app.stack = app.stack.slice(0, -1);
    return true;
  }
  return false;
}

export function toast(text: string) {
  app.toast = { text, at: Date.now() };
}

export function openMenu(title: string, items: MenuItem[]) {
  app.menu = { title, items };
}

export async function loadHome(force = false) {
  if (!app.signedIn) return;
  app.homeLoading = true;
  try {
    app.home = await api.home();
    app.homeError = null;
    app.offline = false;
  } catch (e) {
    app.homeError = String(e);
    if (force) toast(`Couldn't refresh: ${e}`);
  } finally {
    app.homeLoading = false;
  }
}

export async function init() {
  const boot = await api.bootstrap();
  api.launchInfo().then((l) => (app.launch = l)).catch(() => {});
  app.config = boot.config;
  app.signedIn = boot.signedIn;
  app.player = boot.player;
  setBindings(boot.config.bindings as never);
  document.documentElement.dataset.theme = boot.config.theme || "bigpicture";

  await on<PlayerSnapshot>("player-state", (p) => {
    app.player = p;
  });
  await on<Progress>("player-progress", (pr) => {
    const p = app.player;
    if (!p) return;
    p.position = pr.position;
    p.duration = pr.duration;
    p.paused = pr.paused;
    p.index = pr.index;
  });
  await on<number>("player-volume", (v) => {
    if (app.player) app.player.volume = v;
    app.volumeOsd = Date.now();
  });
  window.addEventListener("toggle-fullscreen", async () => {
    if (!isTauri) return;
    const w = getCurrentWindow();
    const fs = !(await w.isFullscreen());
    await w.setFullscreen(fs);
    app.config = await api.saveSettings({ fullscreen: fs });
  });

  if (app.signedIn) {
    // Show cached Home immediately, refresh in the background.
    app.home = await api.homeCached();
    app.ready = true;
    loadHome();
    api.checkConnection().catch((e) => {
      app.offline = true;
      toast(`Can't reach server: ${e}`);
    });
  } else {
    app.ready = true;
  }
}

export function currentSong() {
  const p = app.player;
  return p ? p.queue[p.index] : undefined;
}
