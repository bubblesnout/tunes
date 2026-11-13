// Library lists for the main-menu pages (Artists, Albums, Playlists, Genres).
// Kept in memory for the session: a revisit shows the last list at once and refreshes it
// in the background.

import { api, type Album, type Artist, type Genre, type Playlist } from "./api";

/** Navidrome's default; replaced by the server's own list once artists have loaded. */
let articles = "The El La Los Las Le Les Os As O A";

/** Sort key: lower case, accents stripped, leading article ("The ") dropped. */
export function sortKey(name: string): string {
  let k = name.normalize("NFD").replace(/[̀-ͯ]/g, "").trim().toLowerCase();
  for (const a of articles.toLowerCase().split(/\s+/).filter(Boolean)) {
    if (k.startsWith(a + " ")) {
      k = k.slice(a.length + 1).trimStart();
      break;
    }
  }
  return k;
}

/** Index letter for a sort key: A–Z, or "#" for anything else. */
export function letterOf(key: string): string {
  const c = key.charAt(0).toUpperCase();
  return c >= "A" && c <= "Z" ? c : "#";
}

/** Sort by name (articles ignored) and tag each item with its index letter; "#" first, as on the server. */
function byName<T>(list: T[], name: (t: T) => string): (T & { letter: string })[] {
  return list
    .map((t) => {
      const key = sortKey(name(t));
      return { ...t, letter: letterOf(key), key };
    })
    .sort((a, b) => (a.letter === "#") !== (b.letter === "#") ? (a.letter === "#" ? -1 : 1) : a.key.localeCompare(b.key))
    .map(({ key: _k, ...t }) => t as T & { letter: string });
}

export const library = $state({
  albumArtists: null as (Artist & { letter: string })[] | null,
  artists: null as (Artist & { letter: string })[] | null,
  albums: null as (Album & { letter: string })[] | null,
  playlists: null as (Playlist & { letter: string })[] | null,
  genres: null as (Genre & { letter: string })[] | null,
  error: null as string | null,
});

export type ListKind = "albumArtists" | "artists" | "albums" | "playlists" | "genres";

const loaders: Record<ListKind, () => Promise<unknown[]>> = {
  // getArtists: album artists only (Navidrome), already sorted and grouped by the server.
  albumArtists: async () => {
    const idx = await api.artists();
    if (idx.ignoredArticles) articles = idx.ignoredArticles;
    // The server has already sorted and grouped artists, honouring its ignored articles.
    return idx.artists.map((a) => ({ ...a, letter: a.letter && /^[A-Z]$/.test(a.letter) ? a.letter : "#" }));
  },
  // Track artists: everyone credited as "artist" on a song. Servers without OpenSubsonic roles
  // give no roles at all, in which case everyone search3 returns is kept.
  artists: async () => {
    const all = await api.allArtists();
    const anyRoles = all.some((a) => a.roles?.length);
    return byName(anyRoles ? all.filter((a) => a.roles?.includes("artist")) : all, (a) => a.name);
  },
  albums: async () => byName(await api.albums(), (a) => a.name),
  playlists: async () => byName(await api.playlists(), (p) => p.name),
  genres: async () => byName((await api.genres()).filter((g) => g.name), (g) => g.name),
};

const inflight = new Map<ListKind, Promise<void>>();

/** Load (or refresh) a list. Concurrent calls share one request. */
export function loadList(kind: ListKind): Promise<void> {
  let p = inflight.get(kind);
  if (!p) {
    p = loaders[kind]()
      .then((list) => {
        (library as Record<ListKind, unknown>)[kind] = list;
        library.error = null;
      })
      .catch((e) => {
        library.error = String(e);
      })
      .finally(() => inflight.delete(kind));
    inflight.set(kind, p);
  }
  return p;
}

/** Albums in one genre, sorted and lettered like the Albums page. */
export async function genreAlbums(genre: string) {
  return byName(await api.genreAlbums(genre), (a) => a.name);
}

/** Forget everything (e.g. after signing out). */
export function clearLibrary() {
  library.albumArtists = library.artists = library.albums = library.playlists = library.genres = null;
}
