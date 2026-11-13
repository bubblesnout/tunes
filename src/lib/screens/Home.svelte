<script lang="ts">
  import { api } from "../api";
  import { app, currentSong, loadHome, navigate } from "../app.svelte";
  import { albumMenu, playlistMenu } from "../actions";
  import { ensureFocus } from "../nav";
  import Card from "../components/Card.svelte";
  import Shelf from "../components/Shelf.svelte";
  import ResumeCard from "../components/ResumeCard.svelte";
  import { onMount, tick } from "svelte";

  const TITLES: Record<string, string> = {
    resume: "Continue Listening",
    recentlyAdded: "Recently Added",
    recentlyPlayed: "Recently Played",
    mostPlayed: "Most Played",
    starred: "Favourites",
    random: "Discover",
    highestRated: "Top Rated",
    playlists: "Playlists",
  };

  const sections = $derived((app.config?.homeSections ?? []).filter((s) => s.enabled));
  const resumeSong = $derived(currentSong());
  const visible = $derived(
    sections.filter((s) =>
      s.id === "resume" ? !!resumeSong : s.id === "playlists" ? app.home.playlists.length > 0 : (app.home.albums[s.id]?.length ?? 0) > 0,
    ),
  );
  // Like Big Picture's "Recent Games" row: the first album/playlist shelf gets a wide hero tile.
  // (Continue Listening is a compact single card and never takes the hero.)
  const firstId = $derived(visible.find((s) => s.id !== "resume")?.id);

  onMount(() => {
    tick().then(ensureFocus);
    return () => (app.backdrop = null);
  });
  $effect(() => {
    app.home;
    tick().then(ensureFocus);
  });

  async function resume() {
    await api.player("play");
    navigate({ name: "nowPlaying" });
  }
</script>

<div class="home page">
  {#each visible as s, si (s.id)}
    {#if s.id === "resume" && resumeSong}
      <Shelf title={TITLES.resume} compact>
        <ResumeCard initial={si === 0} onselect={resume} />
      </Shelf>
    {:else if s.id === "playlists"}
      <Shelf title={TITLES.playlists}>
        {#each app.home.playlists as p, i (p.id)}
          <Card hero={s.id === firstId && i === 0} initial={si === 0 && i === 0} title={p.name} subtitle={`${p.songCount ?? 0} tracks`} kicker="Playlist" cover={p.coverArt}
            onselect={() => navigate({ name: "playlist", id: p.id })} onoptions={() => playlistMenu(p)} />
        {/each}
      </Shelf>
    {:else}
      <Shelf title={TITLES[s.id] ?? s.id}>
        {#each app.home.albums[s.id] as a, i (a.id)}
          <Card hero={s.id === firstId && i === 0} initial={si === 0 && i === 0} title={a.name} subtitle={a.artist} kicker={TITLES[s.id]}
            cover={a.coverArt} onselect={() => navigate({ name: "album", id: a.id })} onoptions={() => albumMenu(a)} />
        {/each}
      </Shelf>
    {/if}
  {/each}

  {#if !visible.length}
    <div class="empty">
      {#if app.homeLoading}
        <p class="dim">Loading your library…</p>
      {:else if app.homeError}
        <p>Couldn't load your library.</p>
        <p class="dim">{app.homeError}</p>
        <button class="btn" data-nav onclick={() => loadHome(true)}>Retry</button>
      {:else}
        <p class="dim">Nothing here yet. Enable some sections in Settings.</p>
      {/if}
    </div>
  {/if}
</div>

<style>
  .home { padding: 4px 0 24px; }
  .empty { padding: 80px var(--gutter); display: flex; flex-direction: column; gap: 12px; align-items: flex-start; }
  .empty p { margin: 0; }
</style>
