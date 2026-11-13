// Browser-only mock backend so the UI can be developed/previewed with `npm run dev`
// outside Tauri. Never loaded inside the real app.

import type { Album, Artist, Playlist, PlayerSnapshot, Song } from "./api";

const ARTISTS = ["Aurora Fields", "The Midnight Arcade", "Kōji Sato", "Neon Harbor", "Velvet Static", "Lumen Drift", "Paper Moons", "Glasshouse"];
const WORDS = ["Echoes", "Satellite", "Golden Hour", "Nightdrive", "Low Tide", "Paper Hearts", "Afterglow", "Northern", "Static Bloom", "Overpass", "Polaroid", "Wildfire", "Monochrome", "Fever Dream", "Sundial", "Hologram"];

function rand(seed: number) {
  return () => ((seed = (seed * 16807) % 2147483647) / 2147483647);
}

function albums(seed: number, n: number): Album[] {
  const r = rand(seed);
  return Array.from({ length: n }, (_, i) => ({
    id: `al-${seed}-${i}`,
    name: `${WORDS[Math.floor(r() * WORDS.length)]}${r() > 0.6 ? " " + WORDS[Math.floor(r() * WORDS.length)] : ""}`,
    artist: ARTISTS[Math.floor(r() * ARTISTS.length)],
    coverArt: `al-${seed}-${i}`,
    year: 1995 + Math.floor(r() * 30),
    songCount: 8 + Math.floor(r() * 8),
    duration: 1800 + Math.floor(r() * 2400),
  }));
}

function songsFor(a: Album): Song[] {
  const r = rand(a.id.length * 97 + a.name.length);
  return Array.from({ length: a.songCount ?? 10 }, (_, i) => ({
    id: `${a.id}-s${i}`,
    title: `${WORDS[Math.floor(r() * WORDS.length)]}${r() > 0.5 ? " (" + WORDS[Math.floor(r() * WORDS.length)] + ")" : ""}`,
    album: a.name,
    albumId: a.id,
    artist: a.artist,
    coverArt: a.coverArt,
    duration: 150 + Math.floor(r() * 200),
    track: i + 1,
    year: a.year,
  }));
}

const HOME = {
  albums: {
    recentlyAdded: albums(11, 16),
    recentlyPlayed: albums(23, 14),
    mostPlayed: albums(37, 14),
    random: albums(41, 14),
    starred: albums(53, 10),
    highestRated: albums(67, 10),
  } as Record<string, Album[]>,
  playlists: [
    { id: "pl-1", name: "Late Night Drive", coverArt: "pl-1", songCount: 42 },
    { id: "pl-2", name: "Focus", coverArt: "pl-2", songCount: 18 },
    { id: "pl-3", name: "Road Trip", coverArt: "pl-3", songCount: 77 },
  ] as Playlist[],
};

// A larger library for the Artists / Albums pages, so fast scrolling has something to scroll.
const FIRST = ["Amber", "Blue", "Crimson", "Delta", "Echo", "Faded", "Glass", "Hollow", "Iron", "Jade", "Kite", "Lunar", "Marble",
  "Night", "Ocean", "Paper", "Quiet", "Rust", "Silver", "Tidal", "Ultra", "Velvet", "Wild", "Xenon", "Yellow", "Zero", "The Black", "4th"];
const SECOND = ["Harbor", "Lights", "Season", "Arcade", "Fields", "Signal", "Avenue", "Machines", "Garden", "Tape", "Parade", "Motel"];
const GENRES = ["Electronic", "Indie Rock", "Synthwave", "Ambient", "Jazz", "Hip-Hop", "Folk", "Metal"];
const LIB_ARTISTS: Artist[] = FIRST.flatMap((f, i) =>
  SECOND.slice(0, 2 + (i % 4)).map((s, j) => ({ id: `ar-${i}-${j}`, name: `${f} ${s}`, coverArt: `ar-${i}-${j}`, albumCount: 1 + ((i + j) % 4) })),
);
// Featured / guest artists: credited on songs but never an album artist.
const GUESTS: Artist[] = ["Mira Sol", "DJ Kestrel", "Oona Vale", "The Quiet Choir", "Zed Harlow", "Ines Moreau"].map((name, i) => ({
  id: `gu-${i}`, name, coverArt: `gu-${i}`, albumCount: 0, roles: ["artist"],
}));
const LIB_ALBUMS: Album[] = LIB_ARTISTS.flatMap((ar, i) => {
  const r = rand(i + 7);
  return Array.from({ length: ar.albumCount ?? 1 }, (_, k) => ({
    id: `la-${i}-${k}`, name: `${FIRST[Math.floor(r() * FIRST.length)]} ${WORDS[Math.floor(r() * WORDS.length)]}`,
    artist: ar.name, artistId: ar.id, coverArt: `la-${i}-${k}`, year: 1990 + Math.floor(r() * 35), songCount: 8 + Math.floor(r() * 6),
    genre: GENRES[Math.floor(r() * GENRES.length)],
  }));
}).sort((a, b) => a.name.localeCompare(b.name));
const LIB_PLAYLISTS: Playlist[] = [
  ...HOME.playlists,
  ...["Workout", "Chill Evenings", "Coding", "Sunday Morning", "Party", "Rainy Day", "Throwbacks", "Discoveries", "Instrumental", "Gym"].map((name, i) => ({
    id: `pl-x${i}`, name, coverArt: `pl-x${i}`, songCount: 10 + i * 7,
  })),
];

const allAlbums = () => [...Object.values(HOME.albums).flat(), ...LIB_ALBUMS];

function artistIndex() {
  const key = (n: string) => n.replace(/^the\s+/i, "");
  const sorted = [...LIB_ARTISTS].sort((a, b) => key(a.name).localeCompare(key(b.name)));
  return {
    ignoredArticles: "The El La Los Las Le Les",
    artists: sorted.map((a) => ({ ...a, letter: /^[a-z]/i.test(key(a.name)) ? key(a.name)[0].toUpperCase() : "#" })),
  };
}
const firstAlbum = HOME.albums.recentlyPlayed[0];

let player: PlayerSnapshot = {
  queue: songsFor(firstAlbum),
  index: 2,
  paused: true,
  ended: false,
  position: 64,
  duration: songsFor(firstAlbum)[2].duration ?? 200,
  volume: 0.7,
  repeat: "off",
  shuffle: false,
  error: null,
};

const mockSteam = { installed: true, running: true, added: false, account: "demo" };

const listeners = new Map<string, Set<(p: unknown) => void>>();
export function mockListen(event: string, cb: (p: unknown) => void) {
  if (!listeners.has(event)) listeners.set(event, new Set());
  listeners.get(event)!.add(cb);
  return () => listeners.get(event)?.delete(cb);
}
function emit(event: string, payload: unknown) {
  listeners.get(event)?.forEach((cb) => cb(payload));
}

setInterval(() => {
  if (player.paused) return;
  player.position += 0.5;
  if (player.position >= player.duration) {
    player.index = Math.min(player.index + 1, player.queue.length - 1);
    player.position = 0;
    player.duration = player.queue[player.index].duration ?? 200;
    emit("player-state", structuredClone(player));
  } else emit("player-progress", { ...player });
}, 500);

const config = {
  serverUrl: "https://music.example.com",
  username: "demo",
  artistsView: "albumArtist",
  menuSections: ["artists", "albums", "playlists", "genres", "favourites"].map((id) => ({ id, enabled: id !== "genres" })),
  homeSections: ["resume", "recentlyAdded", "recentlyPlayed", "mostPlayed", "starred", "random", "highestRated", "playlists"].map((id, i) => ({ id, enabled: i < 6 || id === "playlists" })),
  bindings: null,
  volume: 0.7,
  theme: "bigpicture",
  fullscreen: true,
  steamOverlay: true,
};

export async function mockInvoke(cmd: string, args: Record<string, any> = {}): Promise<any> {
  await new Promise((r) => setTimeout(r, 60));
  switch (cmd) {
    case "bootstrap":
      return { signedIn: true, config, player: structuredClone(player) };
    case "get_home_cached":
    case "get_home":
      return HOME;
    case "get_album": {
      const a = allAlbums().find((x) => x.id === args.id) ?? firstAlbum;
      return { ...a, song: songsFor(a) };
    }
    case "get_playlist": {
      const p = HOME.playlists.find((x) => x.id === args.id) ?? HOME.playlists[0];
      return { ...p, owner: "demo", entry: allAlbums().slice(0, 6).flatMap((a) => songsFor(a).slice(0, 3)) };
    }
    case "get_artists":
      return artistIndex();
    case "get_all_artists":
      return [...LIB_ARTISTS.map((a, i) => ({ ...a, roles: i % 7 === 3 ? ["albumartist"] : ["albumartist", "artist"] })), ...GUESTS];
    case "get_artist": {
      const a = [...LIB_ARTISTS, ...GUESTS].find((x) => x.id === args.id) ?? LIB_ARTISTS[0];
      return { ...a, album: LIB_ALBUMS.filter((x) => x.artistId === a.id) };
    }
    case "get_artist_songs": {
      // Each guest features on a few albums; album artists also guest on one other album.
      const n = Number(String(args.id).split("-")[1]) || 0;
      return LIB_ALBUMS.filter((_, i) => i % 23 === n % 23).slice(0, 4)
        .map((a) => ({ ...songsFor(a)[1], artists: [{ id: args.id, name: args.name }] }));
    }
    case "get_albums":
      return LIB_ALBUMS;
    case "get_playlists":
      return LIB_PLAYLISTS;
    case "get_genres":
      return GENRES.map((name) => ({ name, albumCount: LIB_ALBUMS.filter((a) => a.genre === name).length, songCount: 0 }));
    case "get_genre_albums":
      return LIB_ALBUMS.filter((a) => a.genre === args.genre);
    case "get_starred":
      return {
        artist: LIB_ARTISTS.slice(3, 9),
        album: HOME.albums.starred,
        song: HOME.albums.starred.slice(0, 4).flatMap((a) => songsFor(a).slice(0, 3)),
      };
    case "player_play_songs":
      player = { ...player, queue: args.songs, index: args.index, position: 0, paused: false, duration: args.songs[args.index].duration };
      emit("player-state", structuredClone(player));
      return;
    case "player_enqueue":
      player.queue.splice(args.next ? player.index + 1 : player.queue.length, 0, ...args.songs);
      emit("player-state", structuredClone(player));
      return;
    case "player_command": {
      const v = args.value;
      switch (args.cmd) {
        case "toggle": player.paused = !player.paused; break;
        case "play": player.paused = false; break;
        case "pause": player.paused = true; break;
        case "next": player.index = Math.min(player.index + 1, player.queue.length - 1); player.position = 0; break;
        case "prev": player.index = Math.max(player.index - 1, 0); player.position = 0; break;
        case "jump": player.index = v; player.position = 0; break;
        case "volume": player.volume = Math.max(0, Math.min(1, Math.round(v * 100) / 100)); emit("player-volume", player.volume); break;
        case "volumeBy": player.volume = Math.max(0, Math.min(1, Math.round((player.volume + v) * 100) / 100)); emit("player-volume", player.volume); break;
        case "shuffle": player.shuffle = !player.shuffle; break;
        case "repeat": player.repeat = (["off", "all", "one"] as const)[v]; break;
      }
      player.duration = player.queue[player.index].duration ?? 200;
      emit("player-state", structuredClone(player));
      return structuredClone(player);
    }
    case "player_state":
      return structuredClone(player);
    case "save_settings":
      Object.assign(config, Object.fromEntries(Object.entries(args.patch).filter(([, v]) => v !== undefined && v !== null)));
      return config;
    case "launch_info":
      return { launchedBySteam: false, overlayCompat: false };
    case "steam_status":
      return { ...mockSteam };
    case "steam_shortcut":
      mockSteam.added = !!args.add;
      return { ...mockSteam };
    case "steam_restart":
      return;
    case "gamepad_status":
      return { backend: "mock", pads: ["Mock Controller"], focused: true, lastInput: null };
    case "check_connection":
    case "quit":
      return;
    case "logout":
      return { ...config, username: "" };
    default:
      console.warn("mock: unhandled", cmd, args);
  }
}

/** Generated placeholder artwork (gradient + initials) for mock covers. */
export function mockCover(id: string): string {
  let h = 2166136261;
  for (const c of id) h = Math.imul(h ^ c.charCodeAt(0), 16777619) >>> 0;
  h = Math.imul(h ^ (h >>> 15), 2246822507) >>> 0;
  const hue = h % 360;
  const hue2 = (hue + 40 + (h % 80)) % 360;
  const shape = h % 3;
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100"><defs><linearGradient id="g" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="hsl(${hue},70%,55%)"/><stop offset="1" stop-color="hsl(${hue2},65%,22%)"/></linearGradient></defs><rect width="100" height="100" fill="url(#g)"/>${
    shape === 0
      ? `<circle cx="${30 + (h % 40)}" cy="${30 + ((h >> 3) % 40)}" r="${18 + (h % 14)}" fill="hsla(${hue2},90%,85%,.35)"/>`
      : shape === 1
        ? `<rect x="15" y="${20 + (h % 30)}" width="70" height="${6 + (h % 10)}" fill="hsla(${hue},90%,90%,.4)"/><rect x="15" y="${45 + (h % 20)}" width="45" height="4" fill="hsla(${hue},90%,90%,.3)"/>`
        : `<path d="M0 ${60 + (h % 30)} Q50 ${20 + (h % 40)} 100 ${55 + (h % 35)} V100 H0z" fill="hsla(${hue2},80%,70%,.3)"/>`
  }</svg>`;
  return `data:image/svg+xml;utf8,${encodeURIComponent(svg)}`;
}
