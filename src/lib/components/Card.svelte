<script lang="ts">
  // Big Picture style tile. `hero` renders the wide feature tile used for the first item of
  // the top shelf. Focus: quick springy scale + light border; label fades in beneath.
  import { app } from "../app.svelte";
  import { coverUrl } from "../api";
  import Cover from "./Cover.svelte";

  let {
    title, subtitle = "", cover, hero = false, onselect, onoptions, badge = "", kicker = "", initial = false,
  }: {
    title: string; subtitle?: string; cover?: string | null; hero?: boolean; badge?: string; kicker?: string; initial?: boolean;
    onselect: () => void; onoptions?: () => void;
  } = $props();
</script>

<button class="card" class:hero data-nav data-nav-default={initial ? "" : undefined} onclick={onselect}
  onfocus={() => (app.backdrop = cover ?? null)}
  oncontextmenu={(e) => { e.preventDefault(); onoptions?.(); }}
  onnavoptions={() => onoptions?.()}>
  <div class="frame">
    {#if hero}
      <div class="hero-bg" style:background-image={cover ? `url("${coverUrl(cover, 300)}")` : "none"}></div>
      <div class="hero-art"><Cover id={cover} size={600} alt={title} /></div>
      <div class="hero-text">
        {#if kicker}<div class="kicker">{kicker}</div>{/if}
        <div class="hero-title">{title}</div>
        {#if subtitle}<div class="hero-sub">{subtitle}</div>{/if}
      </div>
    {:else}
      <Cover id={cover} size={300} alt={title} />
    {/if}
    {#if badge}<span class="badge">{badge}</span>{/if}
  </div>
  <div class="caption">
    <div class="title ellipsis">{title}</div>
    {#if subtitle}<div class="sub ellipsis">{subtitle}</div>{/if}
  </div>
</button>

<style>
  .card {
    position: relative; flex: 0 0 var(--tile); width: var(--tile); text-align: left;
    /* Room for the caption under the tile without shifting layout */
    padding-bottom: 54px; margin-bottom: -54px;
    z-index: 0;
  }
  .card:focus { z-index: 2; }
  .card.hero { flex-basis: calc(var(--tile) * 2 + var(--tile-gap)); width: calc(var(--tile) * 2 + var(--tile-gap)); }

  .frame {
    position: relative; height: var(--tile); border-radius: var(--radius-sm); overflow: hidden;
    background: var(--surface-3);
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.35);
    transform-origin: 50% 50%;
    transition: transform var(--t-focus) var(--ease-spring), box-shadow var(--t-focus) var(--ease-out);
  }
  .frame::after {
    content: ""; position: absolute; inset: 0; border-radius: inherit; pointer-events: none;
    box-shadow: inset 0 0 0 2px var(--focus-border); opacity: 0;
    transition: opacity calc(var(--t-focus) * 0.6) linear;
  }
  .card:focus .frame { transform: scale(var(--tile-scale)); box-shadow: var(--focus-shadow); }
  .card:focus .frame::after { opacity: 1; }
  .card.hero:focus .frame { transform: scale(1.05); }

  /* Caption appears only for the focused tile, as in Big Picture's shelves */
  .caption {
    position: absolute; left: 0; right: -40%; top: calc(var(--tile) + 14px);
    opacity: 0; transform: translateY(-4px);
    transition: opacity var(--t-focus) var(--ease-out), transform var(--t-focus) var(--ease-out);
    pointer-events: none;
  }
  .card:focus .caption { opacity: 1; transform: translateY(calc(var(--tile) * 0.05)); }
  .card.hero:focus .caption { transform: translateY(calc(var(--tile) * 0.025)); }
  .title { font-size: 15px; font-weight: 600; color: var(--text); }
  .sub { font-size: 11px; margin-top: 3px; color: var(--text-dim); font-weight: 700; letter-spacing: 0.06em; text-transform: uppercase; }

  .hero-bg {
    position: absolute; inset: -20px; background-size: cover; background-position: center;
    filter: blur(28px) saturate(1.4) brightness(0.55); transform: scale(1.1);
  }
  .hero-art { position: absolute; top: 0; bottom: 0; left: 0; width: var(--tile); box-shadow: 8px 0 24px rgba(0,0,0,.45); }
  .hero-art :global(.cover) { border-radius: 0; }
  .hero-text {
    position: absolute; left: calc(var(--tile) + 22px); right: 18px; bottom: 18px; display: flex; flex-direction: column; gap: 4px;
  }
  .kicker { font-size: 11px; font-weight: 700; letter-spacing: 0.08em; text-transform: uppercase; color: var(--green); }
  .hero-title { font-size: clamp(18px, 1.5vw, 28px); font-weight: 700; line-height: 1.15; text-shadow: 0 2px 8px rgba(0,0,0,.5);
    display: -webkit-box; -webkit-line-clamp: 3; line-clamp: 3; -webkit-box-orient: vertical; overflow: hidden; }
  .hero-sub { font-size: 14px; color: var(--text-2); }
  .card.hero .caption { display: none; }

  .badge {
    position: absolute; left: 8px; top: 8px; padding: 3px 8px; border-radius: var(--radius-sm);
    background: var(--green); color: #0e141b; font-size: 11px; font-weight: 800; letter-spacing: .06em; text-transform: uppercase;
  }
</style>
