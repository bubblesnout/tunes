<script lang="ts" module>
  // Last focused tile per page, so coming back from an album returns to where you were.
  const lastFocus = new Map<string, string>();
</script>

<script lang="ts">
  // A–Z grid pages from the main menu: Artists, Albums, Playlists, Genres, and one genre's albums.
  // Hold up/down to fast-scroll with the big letter overlay (fastscroll.svelte.ts).
  import type { Album } from "../api";
  import { app, artistsTitle, navigate, toast, LIBRARY_PAGES } from "../app.svelte";
  import type { Artist } from "../api";
  import { albumMenu, artistMenu, playlistMenu } from "../actions";
  import { genreAlbums, library, loadList, type ListKind } from "../library.svelte";
  import { ensureFocus, focusEl } from "../nav";
  import GridTile from "../components/GridTile.svelte";
  import { onMount, tick } from "svelte";

  let { kind, genre = "" }: { kind: ListKind | "genre"; genre?: string } = $props();

  interface Tile {
    id: string; title: string; subtitle?: string; cover?: string; letter: string; text?: boolean;
    onselect: () => void; onoptions?: () => void;
  }

  let genreList = $state<(Album & { letter: string })[] | null>(null);
  let loading = $state(false);

  const albumTile = (a: Album & { letter: string }): Tile => ({
    id: a.id, title: a.name, subtitle: a.artist, cover: a.coverArt, letter: a.letter,
    onselect: () => navigate({ name: "album", id: a.id }), onoptions: () => albumMenu(a),
  });

  const artistTile = (a: Artist & { letter: string }): Tile => ({
    id: a.id, title: a.name, subtitle: a.albumCount ? `${a.albumCount} album${a.albumCount === 1 ? "" : "s"}` : "",
    cover: a.coverArt, letter: a.letter,
    onselect: () => navigate({ name: "artist", id: a.id }), onoptions: () => artistMenu(a),
  });

  const tiles = $derived.by((): Tile[] | null => {
    switch (kind) {
      case "albumArtists":
        return library.albumArtists?.map(artistTile) ?? null;
      case "artists":
        return library.artists?.map(artistTile) ?? null;
      case "albums":
        return library.albums?.map(albumTile) ?? null;
      case "playlists":
        return library.playlists?.map((p) => ({
          id: p.id, title: p.name, subtitle: `${p.songCount ?? 0} tracks`, cover: p.coverArt, letter: p.letter,
          onselect: () => navigate({ name: "playlist", id: p.id }), onoptions: () => playlistMenu(p),
        })) ?? null;
      case "genres":
        return library.genres?.map((g) => ({
          id: g.name, title: g.name, subtitle: g.albumCount ? `${g.albumCount} album${g.albumCount === 1 ? "" : "s"}` : "",
          letter: g.letter, text: true,
          onselect: () => navigate({ name: "genre", id: g.name }),
        })) ?? null;
      case "genre":
        return genreList?.map(albumTile) ?? null;
    }
  });

  const title = $derived(
    kind === "genre" ? genre : kind === "artists" || kind === "albumArtists" ? artistsTitle(kind) : LIBRARY_PAGES[kind].label,
  );
  const noun = $derived(({ albumArtists: "artist", artists: "artist", albums: "album", playlists: "playlist", genres: "genre", genre: "album" } as const)[kind]);
  const count = $derived(tiles ? `${tiles.length.toLocaleString()} ${noun}${tiles.length === 1 ? "" : "s"}` : "");

  onMount(() => {
    app.backdrop = null;
    load();
    return () => (app.backdrop = null);
  });

  async function load() {
    loading = true;
    try {
      if (kind === "genre") genreList = await genreAlbums(genre);
      else await loadList(kind);
    } catch (e) {
      toast(String(e));
    } finally {
      loading = false;
    }
  }

  const pageKey = $derived(kind === "genre" ? `genre:${genre}` : kind);

  // Once tiles are on screen, focus the one you last left from (or the first).
  let restored = false;
  $effect(() => {
    if (!tiles?.length || restored) return;
    restored = true;
    const id = lastFocus.get(pageKey);
    tick().then(() => {
      const el = id ? document.querySelector<HTMLElement>(`.grid [data-id="${CSS.escape(id)}"]`) : null;
      if (el) focusEl(el, true, "instant");
      else ensureFocus();
    });
  });

  function remember(e: FocusEvent) {
    const id = (e.target as HTMLElement).dataset?.id;
    if (id) lastFocus.set(pageKey, id);
  }
</script>

<div class="library page">
  <header>
    <h1>{title}</h1>
    <span class="count label-caps">{count}</span>
  </header>

  {#if tiles?.length}
    <div class="grid" data-nav-group onfocusin={remember}>
      {#each tiles as t, i (t.id)}
        <GridTile id={t.id} title={t.title} subtitle={t.subtitle} cover={t.cover} letter={t.letter} text={t.text} initial={i === 0}
          onselect={t.onselect} onoptions={t.onoptions} />
      {/each}
    </div>
  {:else}
    <div class="empty">
      {#if loading || !tiles}
        <p class="dim">Loading…</p>
      {:else if library.error && kind !== "genre"}
        <p>Couldn't load {title.toLowerCase()}.</p>
        <p class="dim">{library.error}</p>
        <button class="btn" data-nav data-nav-default onclick={load}>Retry</button>
      {:else}
        <p class="dim">Nothing here yet.</p>
      {/if}
    </div>
  {/if}
</div>

<style>
  .library { padding: 0 var(--gutter) 48px; }
  header { display: flex; align-items: baseline; gap: 18px; padding: 6px 0 18px; }
  h1 { margin: 0; font-size: 30px; font-weight: 700; }
  .count { font-size: 12px; }
  .grid {
    display: grid; grid-template-columns: repeat(auto-fill, minmax(var(--tile), 1fr));
    column-gap: calc(var(--tile-gap) * 1.6); row-gap: 26px;
    /* Room for the focused tile's scale at the edges */
    padding: 10px 0;
  }
  .empty { padding: 60px 0; display: flex; flex-direction: column; gap: 12px; align-items: flex-start; }
  .empty p { margin: 0; }
</style>
