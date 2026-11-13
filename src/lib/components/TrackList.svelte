<script lang="ts">
  // Track table used by album, playlist and favourites pages.
  import { app } from "../app.svelte";
  import { fmtTime, type Song } from "../api";
  import { playSongs, songMenu } from "../actions";

  let { songs, numbering = "index" }: { songs: Song[]; numbering?: "track" | "index" } = $props();

  const currentId = $derived(app.player?.queue[app.player.index]?.id);
  const multiDisc = $derived(numbering === "track" && new Set(songs.map((s) => s.discNumber ?? 1)).size > 1);
</script>

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
          oncontextmenu={(e) => { e.preventDefault(); songMenu(s, () => playSongs(songs, i)); }}
          onnavoptions={() => songMenu(s, () => playSongs(songs, i))}>
          <span class="c-num">
            {#if s.id === currentId}<span class="eq"><i></i><i></i><i></i></span>{:else}{numbering === "track" ? (s.track ?? i + 1) : i + 1}{/if}
          </span>
          <span class="t ellipsis">{s.title}</span>
          <span class="a ellipsis">{s.artist ?? ""}</span>
          <span class="c-dur">{fmtTime(s.duration)}</span>
        </button>
      </li>
    {/each}
  </ol>
</div>

<style>
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
