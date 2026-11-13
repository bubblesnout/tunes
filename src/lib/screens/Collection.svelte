<script lang="ts">
  // Album or playlist page, laid out like a Big Picture game page:
  // hero banner → action bar (Play + stats) → content panel.
  import { api, coverUrl, fmtTime, type Song } from "../api";
  import { app, toast } from "../app.svelte";
  import { playSongs, queueSongs, songMenu } from "../actions";
  import { focusDefault } from "../nav";
  import Cover from "../components/Cover.svelte";
  import Icon from "../components/Icon.svelte";
  import { tick } from "svelte";

  let { kind, id }: { kind: "album" | "playlist"; id: string } = $props();

  let title = $state("");
  let artist = $state("");
  let cover = $state<string | undefined>();
  let songs = $state<Song[]>([]);
  let stats = $state<{ label: string; value: string }[]>([]);
  let loading = $state(true);

  $effect(() => {
    load(kind, id);
  });

  async function load(k: typeof kind, i: string) {
    loading = true;
    try {
      if (k === "album") {
        const a = await api.album(i);
        title = a.name;
        artist = a.artist ?? "";
        cover = a.coverArt;
        songs = a.song ?? [];
        stats = [
          { label: "Released", value: a.year ? String(a.year) : "—" },
          { label: "Tracks", value: String(songs.length) },
          { label: "Length", value: fmtTime(a.duration ?? songs.reduce((t, s) => t + (s.duration ?? 0), 0)) },
          ...(a.genre ? [{ label: "Genre", value: a.genre }] : []),
        ];
      } else {
        const p = await api.playlist(i);
        title = p.name;
        artist = p.owner ? `Playlist by ${p.owner}` : "Playlist";
        cover = p.coverArt;
        songs = p.entry ?? [];
        stats = [
          { label: "Tracks", value: String(songs.length) },
          { label: "Length", value: fmtTime(songs.reduce((t, s) => t + (s.duration ?? 0), 0)) },
        ];
      }
      app.backdrop = null;
    } catch (e) {
      toast(String(e));
    } finally {
      loading = false;
      await tick();
      focusDefault();
    }
  }

  const currentId = $derived(app.player?.queue[app.player.index]?.id);
  const multiDisc = $derived(new Set(songs.map((s) => s.discNumber ?? 1)).size > 1);
</script>

<div class="collection page">
  <div class="hero">
    {#if cover}<div class="hero-bg" style:background-image={`url("${coverUrl(cover, 600)}")`}></div>{/if}
    <div class="hero-content">
      <div class="art"><Cover id={cover} size={600} alt={title} /></div>
      <div class="titles">
        <div class="label-caps kind">{kind === "album" ? "Album" : "Playlist"}</div>
        <h1>{title}</h1>
        <div class="artist">{artist}</div>
      </div>
    </div>
  </div>

  <div class="actionbar" data-nav-group>
    <div class="actions">
      <button class="btn play playbig" data-nav data-nav-default data-nav-entry onclick={() => playSongs(songs)} disabled={loading}>
        <Icon name="play" /> Play
      </button>
    </div>
    <div class="stats">
      {#each stats as s}
        <div class="stat"><div class="label-caps">{s.label}</div><div class="value">{s.value}</div></div>
      {/each}
    </div>
    <div class="actions end">
      <button class="btn icon" data-nav onclick={() => playSongs(songs, 0, true)} disabled={loading} title="Shuffle"><Icon name="shuffle" /></button>
      <button class="btn icon" data-nav onclick={() => queueSongs(songs, true, title)} disabled={loading} title="Play next"><Icon name="queue" /></button>
      <button class="btn icon" data-nav onclick={() => queueSongs(songs, false, title)} disabled={loading} title="Add to queue"><Icon name="plus" /></button>
    </div>
  </div>

  <div class="panel">
    <div class="panel-head label-caps">
      <span class="c-num">#</span><span>Title</span><span>Artist</span><span class="c-dur">Length</span>
    </div>
    <ol class="tracks" data-nav-group>
      {#each songs as s, i (s.id + i)}
        {#if multiDisc && (i === 0 || (songs[i - 1].discNumber ?? 1) !== (s.discNumber ?? 1))}
          <li class="disc label-caps">Disc {s.discNumber ?? 1}</li>
        {/if}
        <li>
          <button class="track" class:playing={s.id === currentId} data-nav
            onclick={() => playSongs(songs, i)}
            oncontextmenu={() => songMenu(s, () => playSongs(songs, i))}
            onnavoptions={() => songMenu(s, () => playSongs(songs, i))}>
            <span class="c-num">
              {#if s.id === currentId}<span class="eq"><i></i><i></i><i></i></span>{:else}{kind === "album" ? (s.track ?? i + 1) : i + 1}{/if}
            </span>
            <span class="t ellipsis">{s.title}</span>
            <span class="a ellipsis">{s.artist ?? ""}</span>
            <span class="c-dur">{fmtTime(s.duration)}</span>
          </button>
        </li>
      {/each}
    </ol>
  </div>
</div>

<style>
  .collection { padding-bottom: 40px; }
  .hero { position: relative; height: clamp(260px, 42vh, 460px); overflow: hidden; margin-top: calc(var(--topbar-h) * -1); }
  .hero-bg {
    position: absolute; inset: -80px; background-size: cover; background-position: center 35%;
    filter: blur(36px) saturate(1.5) brightness(0.55); transform: scale(1.15);
  }
  .hero::after { content: ""; position: absolute; inset: 0; background: linear-gradient(180deg, rgba(0,0,0,.1) 40%, rgba(14,17,22,.85) 100%); }
  .hero-content { position: absolute; z-index: 1; left: var(--gutter); right: var(--gutter); bottom: 26px; display: flex; align-items: flex-end; gap: 28px; }
  .art { width: clamp(150px, 15vw, 240px); flex-shrink: 0; border-radius: var(--radius-sm); overflow: hidden; box-shadow: 0 16px 48px rgba(0,0,0,.6); }
  .titles { min-width: 0; padding-bottom: 6px; }
  .kind { color: var(--text-2); }
  h1 { margin: 6px 0 4px; font-size: clamp(28px, 3vw, 52px); line-height: 1.05; font-weight: 700; text-shadow: 0 2px 12px rgba(0,0,0,.5); }
  .artist { font-size: 18px; color: var(--text-2); }

  .actionbar {
    display: flex; align-items: center; gap: 36px; padding: 14px var(--gutter);
    background: rgba(14, 17, 22, 0.85); border-bottom: 1px solid rgba(255,255,255,.04);
  }
  .actions { display: flex; gap: 8px; }
  .actions.end { margin-left: auto; }
  .playbig { min-width: 200px; justify-content: flex-start; font-size: 17px; }
  .stats { display: flex; gap: 32px; }
  .stat .value { font-size: 15px; color: var(--text-2); margin-top: 3px; }

  .panel { margin: 22px var(--gutter) 0; background: rgba(35, 38, 46, 0.55); border-radius: var(--radius-sm); padding: 6px 0 10px; }
  .panel-head, .track { display: grid; grid-template-columns: 56px minmax(0, 1.4fr) minmax(0, 1fr) 72px; gap: 16px; align-items: center; }
  .panel-head { padding: 10px 20px 8px; }
  .tracks { list-style: none; margin: 0; padding: 0; }
  .disc { padding: 16px 20px 6px; }
  .track {
    width: 100%; padding: 11px 20px; text-align: left; font-size: 15px; color: var(--text-2);
    transition: background var(--t-focus) var(--ease-out);
  }
  .track:focus { background: var(--control); color: var(--text); box-shadow: inset 3px 0 0 var(--accent); }
  .a { color: var(--text-dim); }
  .track:focus .a { color: var(--text-2); }
  .c-num { text-align: right; color: var(--text-dim); font-variant-numeric: tabular-nums; }
  .c-dur { text-align: right; color: var(--text-dim); font-variant-numeric: tabular-nums; }
  .track.playing .t { color: var(--accent-2); }
  .eq { display: inline-flex; align-items: flex-end; gap: 2px; height: 14px; }
  .eq i { width: 3px; background: var(--accent-2); animation: eq 900ms ease-in-out infinite; }
  .eq i:nth-child(2) { animation-delay: -300ms; }
  .eq i:nth-child(3) { animation-delay: -600ms; }
  @keyframes eq { 0%, 100% { height: 4px; } 50% { height: 14px; } }
</style>
