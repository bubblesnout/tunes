// Common library actions shared between screens (play, queue, options menus).

import { api, type Album, type Playlist, type Song } from "./api";
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

export function songMenu(s: Song, playAll?: () => void) {
  const items = [
    ...(playAll ? [{ label: "Play from here", run: playAll }] : []),
    { label: "Play next", run: () => queueSongs([s], true, s.title) },
    { label: "Add to queue", run: () => queueSongs([s], false, s.title) },
  ];
  if (s.albumId) items.push({ label: "Go to album", run: () => navigate({ name: "album", id: s.albumId! }) });
  openMenu(s.title, items);
}
