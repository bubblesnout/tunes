<script lang="ts">
  // Artist page: hero, play/shuffle the whole discography, then the albums (newest first).
  import { api, type Album } from "../api";
  import { app, navigate, toast } from "../app.svelte";
  import { albumMenu, appearsOn, artistSongs } from "../actions";
  import { focusDefault } from "../nav";
  import ActionBar from "../components/ActionBar.svelte";
  import GridTile from "../components/GridTile.svelte";
  import Hero from "../components/Hero.svelte";
  import { tick } from "svelte";

  let { id }: { id: string } = $props();

  let name = $state("");
  let cover = $state<string | undefined>();
  let albums = $state<Album[]>([]);
  let appears = $state<Album[]>([]);
  let loading = $state(true);

  $effect(() => {
    load(id);
  });

  async function load(i: string) {
    loading = true;
    try {
      const a = await api.artist(i);
      name = a.name;
      albums = [...(a.album ?? [])].sort((x, y) => (y.year ?? 0) - (x.year ?? 0));
      const credited = await api.artistSongs(i, a.name).catch(() => []);
      appears = appearsOn(credited, new Set(albums.map((x) => x.id)));
      cover = a.coverArt ?? albums[0]?.coverArt ?? appears[0]?.coverArt;
      app.backdrop = null;
    } catch (e) {
      toast(String(e));
    } finally {
      loading = false;
      await tick();
      focusDefault();
    }
  }

  const stats = $derived([
    { label: "Albums", value: String(albums.length) },
    ...(appears.length ? [{ label: "Appears on", value: String(appears.length) }] : []),
  ]);
</script>

<div class="artist page">
  <Hero {cover} kind="Artist" title={name} />
  <ActionBar title={name} {stats} disabled={loading || !(albums.length || appears.length)} songs={() => artistSongs(id, name)} />
  {#each [{ title: "Albums", list: albums }, { title: "Appears On", list: appears }] as section}
    {#if section.list.length}
      <h2>{section.title}</h2>
      <div class="grid" data-nav-group>
        {#each section.list as a (a.id)}
          <GridTile title={a.name} subtitle={section.title === "Albums" ? (a.year ? String(a.year) : "") : (a.artist ?? "")} cover={a.coverArt}
            onselect={() => navigate({ name: "album", id: a.id })} onoptions={() => albumMenu(a)} />
        {/each}
      </div>
    {/if}
  {/each}
  {#if !loading && !albums.length && !appears.length}
    <p class="dim none">No albums found for this artist.</p>
  {/if}
</div>

<style>
  .artist { padding-bottom: 48px; }
  .none { padding: 26px var(--gutter); margin: 0; }
  h2 { margin: 26px var(--gutter) 4px; font-size: 19px; font-weight: 600; color: var(--text-2); }
  .grid {
    display: grid; grid-template-columns: repeat(auto-fill, minmax(var(--tile), 1fr));
    column-gap: calc(var(--tile-gap) * 1.6); row-gap: 26px; padding: 12px var(--gutter);
  }
</style>
