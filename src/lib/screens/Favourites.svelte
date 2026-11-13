<script lang="ts">
  // Everything starred on the server: songs (playable as a list), albums and artists.
  import { api, type Starred } from "../api";
  import { app, navigate, toast } from "../app.svelte";
  import { albumMenu, artistMenu } from "../actions";
  import { focusDefault } from "../nav";
  import ActionBar from "../components/ActionBar.svelte";
  import Card from "../components/Card.svelte";
  import Shelf from "../components/Shelf.svelte";
  import TrackList from "../components/TrackList.svelte";
  import { onMount, tick } from "svelte";

  let starred = $state<Starred | null>(null);

  onMount(() => {
    app.backdrop = null;
    load();
    return () => (app.backdrop = null);
  });

  async function load() {
    try {
      starred = await api.starred();
    } catch (e) {
      toast(String(e));
    }
    await tick();
    focusDefault();
  }

  const songs = $derived(starred?.song ?? []);
</script>

<div class="favourites page">
  <header><h1>Favourites</h1></header>
  {#if starred}
    {#if songs.length}
      <ActionBar title="Favourites" stats={[{ label: "Songs", value: String(songs.length) }]} songs={() => songs} />
    {/if}
    {#if starred.album.length}
      <Shelf title="Albums">
        {#each starred.album as a (a.id)}
          <Card title={a.name} subtitle={a.artist} cover={a.coverArt}
            onselect={() => navigate({ name: "album", id: a.id })} onoptions={() => albumMenu(a)} />
        {/each}
      </Shelf>
    {/if}
    {#if starred.artist.length}
      <Shelf title="Artists">
        {#each starred.artist as a (a.id)}
          <Card title={a.name} cover={a.coverArt}
            onselect={() => navigate({ name: "artist", id: a.id })} onoptions={() => artistMenu(a)} />
        {/each}
      </Shelf>
    {/if}
    {#if songs.length}
      <h2>Songs</h2>
      <TrackList {songs} />
    {/if}
    {#if !songs.length && !starred.album.length && !starred.artist.length}
      <p class="dim empty">Nothing starred yet. Star songs, albums or artists in Navidrome and they'll appear here.</p>
    {/if}
  {:else}
    <p class="dim empty">Loading…</p>
  {/if}
</div>

<style>
  .favourites { padding-bottom: 48px; }
  header { padding: 6px var(--gutter) 18px; }
  h1 { margin: 0; font-size: 30px; font-weight: 700; }
  h2 { margin: 0 var(--gutter); font-size: 19px; font-weight: 600; color: var(--text-2); }
  .favourites :global(.actionbar) { margin-bottom: 22px; }
  .empty { padding: 40px var(--gutter); margin: 0; }
</style>
