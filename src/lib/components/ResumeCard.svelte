<script lang="ts">
  // Compact "Continue Listening" card: one short, wide tile for the current queue.
  import { app, currentSong } from "../app.svelte";
  import { coverUrl } from "../api";
  import Cover from "./Cover.svelte";
  import Icon from "./Icon.svelte";

  let { initial = false, onselect }: { initial?: boolean; onselect: () => void } = $props();

  const song = $derived(currentSong());
  const p = $derived(app.player);
  const pct = $derived(p && p.duration ? Math.min(100, (p.position / p.duration) * 100) : 0);
</script>

{#if song && p}
  <button class="resume" data-nav data-nav-default={initial ? "" : undefined} onclick={onselect}
    onfocus={() => (app.backdrop = song.coverArt ?? null)}>
    <div class="frame">
      <div class="bg" style:background-image={song.coverArt ? `url("${coverUrl(song.coverArt, 300)}")` : "none"}></div>
      <div class="art"><Cover id={song.coverArt} size={300} alt={song.album ?? ""} /></div>
      <div class="text">
        <div class="kicker" class:playing={!p.paused}>
          <Icon name={p.paused ? "play" : "music"} size={12} /> {p.paused ? "Resume" : "Now Playing"}
        </div>
        <div class="title ellipsis">{song.title}</div>
        <div class="sub ellipsis">{song.artist ?? ""}{song.album ? ` — ${song.album}` : ""}</div>
        <div class="meta">Track {p.index + 1} of {p.queue.length}</div>
      </div>
      <div class="progress"><div class="fill" style:width="{pct}%"></div></div>
    </div>
  </button>
{/if}

<style>
  .resume { --h: calc(var(--tile) * 0.62); display: block; width: min(calc(var(--tile) * 3 + var(--tile-gap) * 2), 70vw); text-align: left; }
  .frame {
    position: relative; height: var(--h); border-radius: var(--radius-sm); overflow: hidden; display: flex; align-items: stretch;
    background: var(--surface-3); box-shadow: 0 4px 12px rgba(0, 0, 0, 0.35);
    transform-origin: 0 50%;
    transition: transform var(--t-focus) var(--ease-spring), box-shadow var(--t-focus) var(--ease-out);
  }
  .frame::after {
    content: ""; position: absolute; inset: 0; border-radius: inherit; pointer-events: none;
    box-shadow: inset 0 0 0 2px var(--focus-border); opacity: 0; transition: opacity calc(var(--t-focus) * 0.6) linear;
  }
  .resume:focus .frame { transform: scale(1.04); box-shadow: var(--focus-shadow); }
  .resume:focus .frame::after { opacity: 1; }
  .bg { position: absolute; inset: -30px; background-size: cover; background-position: center; filter: blur(30px) saturate(1.4) brightness(0.45); }
  .art { position: relative; height: 100%; aspect-ratio: 1; flex-shrink: 0; }
  .art :global(.cover) { border-radius: 0; height: 100%; }
  .text { position: relative; flex: 1; min-width: 0; padding: 0 22px; display: flex; flex-direction: column; justify-content: center; gap: 3px; }
  .kicker { display: flex; align-items: center; gap: 6px; font-size: 11px; font-weight: 700; letter-spacing: 0.08em; text-transform: uppercase; color: var(--accent-2); }
  .kicker.playing { color: var(--green); }
  .title { font-size: clamp(16px, 1.3vw, 22px); font-weight: 700; color: var(--text); }
  .sub { font-size: 14px; color: var(--text-2); }
  .meta { font-size: 12px; color: var(--text-dim); margin-top: 2px; }
  .progress { position: absolute; left: 0; right: 0; bottom: 0; height: 3px; background: rgba(255, 255, 255, 0.12); }
  .fill { height: 100%; background: var(--accent); transition: width 500ms linear; }
</style>
