// Common library actions shared between screens (play, queue, options menus).

import { api, type Album, type Artist, type Playlist, type Song } from "./api";
import { app, navigate, openMenu, toast } from "./app.svelte";

async function albumSongs(id: string): Promise<Song[]> {
  return (await api.album(id)).song ?? [];
}

async function playlistSongs(id: string): Promise<Song[]> {
  return (await api.playlist(id)).entry ?? [];
}

export async function playSongs(songs: Song[], index = 0, shuffle = false) {
  if (!songs.length) return toast("Nothing to play");
  if (shuffle !== (app.player?.shuffle ?? false)) await api.player("shuffle");
  await api.playSongs(songs, index);
}

export async function playAlbum(id: string, index = 0, shuffle = false) {
  try {
    await playSongs(await albumSongs(id), index, shuffle);
  } catch (e) {
    toast(String(e));
  }
}

export async function queueSongs(songs: Song[], next: boolean, label: string) {
  if (!songs.length) return;
  await api.enqueue(songs, next);
  toast(next ? `Playing next: ${label}` : `Added to queue: ${label}`);
}

export function albumMenu(a: Album) {
  openMenu(a.name, [
    { label: "Play", run: () => playAlbum(a.id) },
    { label: "Shuffle", run: () => playAlbum(a.id, 0, true) },
    { label: "Play next", run: async () => queueSongs(await albumSongs(a.id), true, a.name) },
    { label: "Add to queue", run: async () => queueSongs(await albumSongs(a.id), false, a.name) },
    { label: "Open album", run: () => navigate({ name: "album", id: a.id }) },
  ]);
}

export function playlistMenu(p: Playlist) {
  openMenu(p.name, [
    { label: "Play", run: async () => playSongs(await playlistSongs(p.id)) },
    { label: "Shuffle", run: async () => playSongs(await playlistSongs(p.id), 0, true) },
    { label: "Play next", run: async () => queueSongs(await playlistSongs(p.id), true, p.name) },
    { label: "Add to queue", run: async () => queueSongs(await playlistSongs(p.id), false, p.name) },
    { label: "Open playlist", run: () => navigate({ name: "playlist", id: p.id }) },
  ]);
}

/** Albums an artist appears on without being the album artist, built from their songs. */
export function appearsOn(songs: Song[], ownAlbumIds: Set<string>): Album[] {
  const out = new Map<string, Album>();
  for (const s of songs) {
    if (!s.albumId || ownAlbumIds.has(s.albumId) || out.has(s.albumId)) continue;
    out.set(s.albumId, { id: s.albumId, name: s.album ?? "", artist: s.artist, coverArt: s.coverArt, year: s.year });
  }
  return [...out.values()].sort((a, b) => (b.year ?? 0) - (a.year ?? 0));
}

/**
 * Every song by an artist: their own albums (newest first), then tracks they appear on
 * elsewhere (Navidrome's getArtist leaves those albums out for track-only credits).
 */
export async function artistSongs(id: string, name: string): Promise<Song[]> {
  const [detail, credited] = await Promise.all([api.artist(id), api.artistSongs(id, name).catch(() => [] as Song[])]);
  const albums = [...(detail.album ?? [])].sort((a, b) => (b.year ?? 0) - (a.year ?? 0));
  const own = (await Promise.all(albums.map((a) => api.album(a.id)))).flatMap((d) => d.song ?? []);
  const ids = new Set(own.map((s) => s.id));
  return [...own, ...credited.filter((s) => !ids.has(s.id))];
}

export function artistMenu(a: Artist) {
  const run = (f: (songs: Song[]) => unknown) => async () => {
    try {
      await f(await artistSongs(a.id, a.name));
    } catch (e) {
      toast(String(e));
    }
  };
  openMenu(a.name, [
    { label: "Play", run: run((s) => playSongs(s)) },
    { label: "Shuffle", run: run((s) => playSongs(s, 0, true)) },
    { label: "Play next", run: run((s) => queueSongs(s, true, a.name)) },
    { label: "Add to queue", run: run((s) => queueSongs(s, false, a.name)) },
    { label: "Open artist", run: () => navigate({ name: "artist", id: a.id }) },
  ]);
}

export function songMenu(s: Song, playAll?: () => void) {
  const items = [
    ...(playAll ? [{ label: "Play from here", run: playAll }] : []),
    { label: "Play next", run: () => queueSongs([s], true, s.title) },
    { label: "Add to queue", run: () => queueSongs([s], false, s.title) },
  ];
  if (s.albumId) items.push({ label: "Go to album", run: () => navigate({ name: "album", id: s.albumId! }) });
  if (s.artistId) items.push({ label: "Go to artist", run: () => navigate({ name: "artist", id: s.artistId! }) });
  openMenu(s.title, items);
}
