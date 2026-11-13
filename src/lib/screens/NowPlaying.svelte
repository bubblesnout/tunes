<script lang="ts">
  import { api, coverUrl, fmtTime } from "../api";
  import { app, navigate } from "../app.svelte";
  import { focusDefault } from "../nav";
  import Cover from "../components/Cover.svelte";
  import Icon from "../components/Icon.svelte";
  import { onMount, tick } from "svelte";

  const p = $derived(app.player);
  const song = $derived(p?.queue[p.index]);
  const upNext = $derived(p ? p.queue.slice(p.index + 1, p.index + 8).map((s, i) => ({ s, i: p.index + 1 + i })) : []);
  const pct = $derived(p && p.duration ? Math.min(100, (p.position / p.duration) * 100) : 0);

  onMount(() => {
    app.backdrop = null;
    tick().then(focusDefault);
  });

  function cycleRepeat() {
    const next = p?.repeat === "off" ? 1 : p?.repeat === "all" ? 2 : 0;
    api.player("repeat", next);
  }
</script>

<div class="np page">
  {#if song?.coverArt}
    {#key song.coverArt}
      <div class="bg" style:background-image={`url("${coverUrl(song.coverArt, 300)}")`}></div>
    {/key}
  {/if}
  {#if song && p}
    <div class="main">
      {#key song.id}
        <div class="art"><Cover id={song.coverArt} size={800} alt={song.album ?? ""} /></div>
      {/key}
      <div class="side">
        <div class="label-caps state" class:playing={!p.paused}>{p.paused ? "Paused" : "Now Playing"}</div>
        <h1>{song.title}</h1>
        <div class="artist">{song.artist ?? ""}</div>
        <button class="album-link" data-nav onclick={() => song.albumId && navigate({ name: "album", id: song.albumId })}>
          {song.album ?? ""}{song.year ? ` · ${song.year}` : ""}
        </button>

        <div class="progress">
          <div class="bar"><div class="fill" style:width="{pct}%"></div></div>
          <div class="times"><span>{fmtTime(p.position)}</span><span>{fmtTime(p.duration)}</span></div>
        </div>

        <div class="controls" data-nav-group>
          <button class="btn icon toggle" class:on={p.shuffle} data-nav onclick={() => api.player("shuffle")} title="Shuffle"><Icon name="shuffle" /></button>
          <button class="btn icon" data-nav onclick={() => api.player("prev")} title="Previous"><Icon name="prev" /></button>
          <!-- Steam colours: green Play (fresh start), blue Resume, neutral Pause -->
          <button class="btn playbtn" class:play={p.paused && (p.ended || p.position < 1)} class:primary={p.paused && !p.ended && p.position >= 1}
            data-nav data-nav-default data-nav-entry onclick={() => api.player("toggle")}>
            <Icon name={p.paused ? "play" : "pause"} /> {p.paused ? (p.ended || p.position < 1 ? "Play" : "Resume") : "Pause"}
          </button>
          <button class="btn icon" data-nav onclick={() => api.player("next")} title="Next"><Icon name="next" /></button>
          <button class="btn icon toggle" class:on={p.repeat !== "off"} data-nav onclick={cycleRepeat} title="Repeat">
            <Icon name={p.repeat === "one" ? "repeatOne" : "repeat"} />
          </button>
        </div>
        {#if p.error}<p class="err">Playback problem: {p.error}</p>{/if}

        <div class="next">
          <div class="label-caps">Up Next</div>
          {#if upNext.length}
            <ol data-nav-group>
              {#each upNext as { s, i } (i)}
                <li>
                  <button class="row" data-nav onclick={() => api.player("jump", i)}>
                    <span class="thumb"><Cover id={s.coverArt} size={96} /></span>
                    <span class="txt"><span class="ellipsis">{s.title}</span><span class="ellipsis sub">{s.artist}</span></span>
                    <span class="dur">{fmtTime(s.duration)}</span>
                  </button>
                </li>
              {/each}
            </ol>
          {:else}
            <p class="dim">End of queue{p.repeat === "all" ? " — repeating from the start" : ""}.</p>
          {/if}
        </div>
      </div>
    </div>
  {:else}
    <div class="nothing">
      <p class="dim">Nothing is playing.</p>
      <button class="btn" data-nav data-nav-default onclick={() => navigate({ name: "home" })}>Browse music</button>
    </div>
  {/if}
</div>

<style>
  .np { position: relative; min-height: 100%; }
  .bg {
    position: fixed; inset: -80px; background-size: cover; background-position: center; z-index: -1;
    filter: blur(80px) saturate(1.4) brightness(0.38); animation: bgin 700ms ease-out both;
  }
  @keyframes bgin { from { opacity: 0; } }
  .main { display: flex; gap: clamp(32px, 4vw, 72px); padding: 18px var(--gutter) 30px; align-items: flex-start; }
  .art {
    width: min(52vh, 560px); flex-shrink: 0; border-radius: var(--radius-sm); overflow: hidden;
    box-shadow: 0 30px 80px rgba(0,0,0,.65); animation: artin 420ms var(--ease-out) both;
  }
  @keyframes artin { from { opacity: 0; transform: scale(0.96); } }
  .side { flex: 1; min-width: 0; display: flex; flex-direction: column; max-width: 820px; padding-top: 8px; }
  .state.playing { color: var(--green); }
  h1 { margin: 8px 0 6px; font-size: clamp(28px, 2.8vw, 46px); line-height: 1.08; font-weight: 700; }
  .artist { font-size: 20px; color: var(--text-2); }
  .album-link { align-self: flex-start; margin: 6px 0 0 -8px; padding: 4px 8px; border-radius: var(--radius-sm); font-size: 15px; color: var(--text-dim); transition: background var(--t-focus), color var(--t-focus); }
  .album-link:focus { background: var(--control); color: var(--text); }
  .progress { margin: 26px 0 22px; }
  .bar { height: 4px; border-radius: 2px; background: rgba(255,255,255,.14); overflow: hidden; }
  .fill { height: 100%; background: var(--accent); transition: width 500ms linear; }
  .times { display: flex; justify-content: space-between; font-size: 13px; margin-top: 8px; color: var(--text-dim); font-variant-numeric: tabular-nums; }
  .controls { display: flex; align-items: center; gap: 8px; }
  .playbtn { min-width: 150px; }
  .toggle { color: var(--text-dim); }
  .toggle.on { color: var(--accent-2); }
  .toggle.on:focus { color: #0b6fc0; }
  .err { color: #ff9a90; margin: 14px 0 0; }
  .next { margin-top: 34px; }
  ol { list-style: none; margin: 8px 0 0; padding: 0; }
  .row { width: 100%; display: flex; align-items: center; gap: 14px; padding: 7px 12px 7px 7px; text-align: left; border-radius: var(--radius-sm); transition: background var(--t-focus) var(--ease-out); }
  .row:focus { background: var(--control); box-shadow: inset 3px 0 0 var(--accent); }
  .thumb { width: 42px; height: 42px; border-radius: var(--radius-sm); overflow: hidden; flex-shrink: 0; }
  .txt { display: flex; flex-direction: column; min-width: 0; flex: 1; font-size: 15px; color: var(--text-2); }
  .row:focus .txt { color: var(--text); }
  .sub { font-size: 13px; color: var(--text-dim); }
  .dur { font-variant-numeric: tabular-nums; font-size: 13px; color: var(--text-dim); }
  .next p { margin: 8px 0 0; }
  .nothing { padding: 120px var(--gutter); display: flex; flex-direction: column; gap: 16px; align-items: flex-start; }
  .nothing p { margin: 0; font-size: 20px; }
</style>
