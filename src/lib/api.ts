import { invoke as tauriInvoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import * as mock from "./mock";

/** False when running in a plain browser (`npm run dev` preview) — a mock backend is used. */
export const isTauri = "__TAURI_INTERNALS__" in window;

function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  return isTauri ? tauriInvoke<T>(cmd, args) : mock.mockInvoke(cmd, args);
}

/** Subscribe to a backend event. Returns an unsubscribe function. */
export async function on<T>(event: string, cb: (payload: T) => void): Promise<() => void> {
  if (isTauri) return listen<T>(event, (e) => cb(e.payload));
  return mock.mockListen(event, cb as (p: unknown) => void);
}

export interface Album {
  id: string;
  name: string;
  artist?: string;
  artistId?: string;
  coverArt?: string;
  year?: number;
  songCount?: number;
  duration?: number;
  genre?: string;
}

export interface Song {
  id: string;
  title: string;
  album?: string;
  albumId?: string;
  artist?: string;
  coverArt?: string;
  duration?: number;
  track?: number;
  discNumber?: number;
  year?: number;
  suffix?: string;
  contentType?: string;
}

export interface AlbumDetail extends Album {
  song: Song[];
}

export interface Playlist {
  id: string;
  name: string;
  coverArt?: string;
  songCount?: number;
  owner?: string;
}

export interface PlaylistDetail extends Playlist {
  entry: Song[];
}

export interface HomeSection {
  id: string;
  enabled: boolean;
}

export interface Config {
  serverUrl: string;
  username: string;
  homeSections: HomeSection[];
  bindings: Record<string, string[]> | null;
  volume: number;
  theme: string;
  fullscreen: boolean;
}

export type Repeat = "off" | "all" | "one";

export interface PlayerSnapshot {
  queue: Song[];
  index: number;
  paused: boolean;
  ended: boolean;
  position: number;
  duration: number;
  volume: number;
  repeat: Repeat;
  shuffle: boolean;
  error: string | null;
}

export interface Progress {
  index: number;
  position: number;
  duration: number;
  paused: boolean;
  volume: number;
}

export interface HomeData {
  albums: Record<string, Album[]>;
  playlists: Playlist[];
}

export interface PadStatus {
  backend: string;
  pads: string[];
  focused: boolean;
  lastInput: string | null;
}

export const api = {
  gamepadStatus: () => invoke<PadStatus>("gamepad_status"),
  bootstrap: () => invoke<{ signedIn: boolean; config: Config; player: PlayerSnapshot }>("bootstrap"),
  login: (server: string, username: string, password: string) =>
    invoke<Config>("login", { server, username, password }),
  logout: () => invoke<Config>("logout"),
  checkConnection: () => invoke<void>("check_connection"),
  homeCached: () => invoke<HomeData>("get_home_cached"),
  home: () => invoke<HomeData>("get_home"),
  album: (id: string) => invoke<AlbumDetail>("get_album", { id }),
  playlist: (id: string) => invoke<PlaylistDetail>("get_playlist", { id }),
  playSongs: (songs: Song[], index = 0) => invoke<void>("player_play_songs", { songs, index }),
  enqueue: (songs: Song[], next: boolean) => invoke<void>("player_enqueue", { songs, next }),
  player: (cmd: string, value?: number) => invoke<PlayerSnapshot>("player_command", { cmd, value }),
  playerState: () => invoke<PlayerSnapshot>("player_state"),
  saveSettings: (patch: Partial<Pick<Config, "homeSections" | "bindings" | "theme" | "fullscreen">>) =>
    invoke<Config>("save_settings", { patch }),
  quit: () => invoke<void>("quit"),
};

const isWindows = navigator.userAgent.includes("Windows");

/** Album art served (and disk-cached) by the backend's `cover://` protocol. */
export function coverUrl(id: string | undefined | null, size = 300): string {
  if (!id) return "";
  if (!isTauri) return mock.mockCover(id);
  const path = `${encodeURIComponent(id)}/${size}`;
  return isWindows ? `http://cover.localhost/${path}` : `cover://localhost/${path}`;
}

export function fmtTime(secs: number | undefined): string {
  if (!secs || !isFinite(secs) || secs < 0) return "0:00";
  const s = Math.floor(secs);
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  const ss = String(s % 60).padStart(2, "0");
  return h ? `${h}:${String(m).padStart(2, "0")}:${ss}` : `${m}:${ss}`;
}
