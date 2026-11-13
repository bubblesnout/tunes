// Global reactive app state (Svelte 5 runes).

import { getCurrentWindow } from "@tauri-apps/api/window";
import { api, isTauri, on, type Config, type HomeData, type PlayerSnapshot, type Progress } from "./api";
import { setBindings, type Device } from "./input";

export type Route =
  | { name: "home" }
  | { name: "album"; id: string }
  | { name: "playlist"; id: string }
  | { name: "nowPlaying" }
  | { name: "settings"; tab?: string };

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
  if (r.name === "home") app.stack = [r];
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
