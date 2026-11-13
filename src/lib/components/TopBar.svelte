<script lang="ts">
  // Minimal Big Picture top bar: status on the right, with Now Playing in the top-right corner.
  import { app, navigate, currentSong, route } from "../app.svelte";
  import Icon from "./Icon.svelte";
  import Cover from "./Cover.svelte";
  import VolumeSlider from "./VolumeSlider.svelte";

  let now = $state(new Date());
  $effect(() => {
    const t = setInterval(() => (now = new Date()), 15_000);
    return () => clearInterval(t);
  });
  const time = $derived(now.toLocaleTimeString([], { hour: "numeric", minute: "2-digit" }));
  const song = $derived(currentSong());
  const pct = $derived(app.player && app.player.duration ? (app.player.position / app.player.duration) * 100 : 0);
  const title = $derived(({ settings: "Settings", nowPlaying: "Now Playing", genre: "Genres" } as Record<string, string>)[route().name] ?? "");
</script>

<header class="top">
  <div class="title">{title}</div>
  <div class="right">
    {#if app.offline}<span class="offline">Offline</span>{/if}
    <span class="vol-wrap">
      <button class="vol" class:open={!!app.volumePopup} data-nav title="Volume"
        onclick={() => (app.volumePopup = app.volumePopup ? 0 : Date.now())}>
        <Icon name={(app.player?.volume ?? 0) === 0 ? "mute" : "volume"} size={17} />{Math.round((app.player?.volume ?? 0) * 100)}
      </button>
      <VolumeSlider />
    </span>
    <span class="clock">{time}</span>
    {#if song}
      <button class="mini" data-nav onclick={() => navigate({ name: "nowPlaying" })} title="Now Playing">
        <span class="mini-text">
          <span class="t ellipsis">{song.title}</span>
          <span class="a ellipsis">{app.player?.paused ? "Paused" : song.artist}</span>
        </span>
        <span class="mini-art">
          <Cover id={song.coverArt} size={96} />
          {#if !app.player?.paused}
            <span class="eq" aria-hidden="true"><i></i><i></i><i></i></span>
          {/if}
        </span>
        <span class="bar" style:width="{pct}%"></span>
      </button>
    {/if}
  </div>
</header>

<style>
  /* The mini player sits in the corner with equal top/right spacing (like Steam's avatar). */
  .top {
    --corner: 8px;
    height: var(--topbar-h); display: flex; align-items: center; padding: 0 var(--corner) 0 var(--gutter);
    flex-shrink: 0; position: relative; z-index: 5;
  }
  .title { font-size: 20px; font-weight: 600; color: var(--text-2); }
  .right { margin-left: auto; display: flex; align-items: center; gap: 20px; color: var(--text-2); font-size: 15px; }
  .vol, .clock { display: flex; align-items: center; gap: 6px; font-size: 15px; font-weight: 500; color: var(--text-2); font-variant-numeric: tabular-nums; }
  .vol-wrap { position: relative; }
  .vol { padding: 6px 10px; margin: 0 -10px; border-radius: var(--radius-sm); transition: background var(--t-focus) var(--ease-out), color var(--t-focus); }
  .vol:hover { background: rgba(255, 255, 255, 0.08); }
  .vol:focus, .vol.open { background: var(--focus-fill); color: var(--focus-ink); }
  .offline { color: var(--warn); font-weight: 600; font-size: 13px; text-transform: uppercase; letter-spacing: .06em; }

  .mini {
    position: relative; display: flex; align-items: center; gap: 12px; max-width: 300px;
    height: calc(var(--topbar-h) - var(--corner) * 2);
    padding: 0 0 0 12px; border-radius: var(--radius-sm); overflow: hidden;
    background: rgba(255, 255, 255, 0.06);
    transition: background var(--t-focus) var(--ease-out), transform var(--t-focus) var(--ease-spring), box-shadow var(--t-focus);
  }
  .mini:hover { background: rgba(255, 255, 255, 0.1); }
  .mini { transform-origin: 100% 50%; }
  .mini:focus { background: var(--focus-fill); color: var(--focus-ink); transform: scale(1.05); box-shadow: var(--focus-shadow); }
  .mini-text { display: flex; flex-direction: column; min-width: 0; text-align: right; line-height: 1.2; }
  .t { font-size: 13px; font-weight: 600; }
  .a { font-size: 11px; color: var(--text-dim); }
  .mini:focus .a { color: #46505c; }
  .mini-art { position: relative; height: 100%; aspect-ratio: 1; flex-shrink: 0; }
  .bar { position: absolute; left: 0; bottom: 0; height: 2px; background: var(--accent); transition: width 500ms linear; }
  .eq { position: absolute; inset: 0; display: flex; align-items: flex-end; justify-content: center; gap: 2px; padding-bottom: 8px; background: rgba(0,0,0,.35); }
  .eq i { width: 3px; background: #fff; border-radius: 1px; animation: eq 900ms ease-in-out infinite; }
  .eq i:nth-child(2) { animation-delay: -300ms; }
  .eq i:nth-child(3) { animation-delay: -600ms; }
  @keyframes eq { 0%, 100% { height: 5px; } 50% { height: 18px; } }
</style>
