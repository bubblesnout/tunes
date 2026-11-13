<script lang="ts">
  // Tile for the library grids (Artists, Albums, Playlists, Genres). Like a shelf Card, but its
  // caption is always shown, and `letter` feeds the A–Z fast scroll.
  import { app } from "../app.svelte";
  import Cover from "./Cover.svelte";

  let {
    id, title, subtitle = "", cover, letter, text = false, initial = false, onselect, onoptions,
  }: {
    id?: string; title: string; subtitle?: string; cover?: string | null; letter?: string;
    /** Show the title on the tile itself instead of artwork (genres). */
    text?: boolean; initial?: boolean;
    onselect: () => void; onoptions?: () => void;
  } = $props();

  // A stable hue per title for text tiles.
  const hue = $derived([...title].reduce((h, c) => (h * 31 + c.charCodeAt(0)) % 360, 7));
</script>

<button class="tile" data-nav data-id={id} data-letter={letter} data-nav-default={initial ? "" : undefined} onclick={onselect}
  onfocus={() => (app.backdrop = cover ?? null)}
  oncontextmenu={(e) => { e.preventDefault(); onoptions?.(); }}
  onnavoptions={() => onoptions?.()}>
  <div class="frame">
    {#if text}
      <div class="text-art" style:--h={hue}><span>{title}</span></div>
    {:else}
      <Cover id={cover} size={300} alt={title} />
    {/if}
  </div>
  <div class="caption">
    {#if !text}<div class="title ellipsis">{title}</div>{/if}
    {#if subtitle}<div class="sub ellipsis">{subtitle}</div>{/if}
  </div>
</button>

<style>
  .tile {
    position: relative; display: block; width: 100%; text-align: left; z-index: 0;
    /* Big libraries: skip rendering tiles that are off screen. */
    content-visibility: auto; contain-intrinsic-size: auto 300px;
  }
  /* content-visibility clips to the box; the focused tile scales past it. */
  .tile:focus { z-index: 2; content-visibility: visible; }
  .frame {
    position: relative; aspect-ratio: 1; border-radius: var(--radius-sm); overflow: hidden;
    background: var(--surface-3); box-shadow: 0 4px 12px rgba(0, 0, 0, 0.35);
    transition: transform var(--t-focus) var(--ease-spring), box-shadow var(--t-focus) var(--ease-out);
  }
  .frame::after {
    content: ""; position: absolute; inset: 0; border-radius: inherit; pointer-events: none;
    box-shadow: inset 0 0 0 2px var(--focus-border); opacity: 0;
    transition: opacity calc(var(--t-focus) * 0.6) linear;
  }
  .tile:focus .frame { transform: scale(var(--tile-scale)); box-shadow: var(--focus-shadow); }
  .tile:focus .frame::after { opacity: 1; }
  .caption { padding: 10px 2px 0; transition: transform var(--t-focus) var(--ease-out); }
  .tile:focus .caption { transform: translateY(calc(var(--tile) * 0.05)); }
  .title { font-size: 14px; font-weight: 500; color: var(--text-2); }
  .tile:focus .title { color: var(--text); }
  .sub { font-size: 11px; margin-top: 3px; color: var(--text-dim); font-weight: 700; letter-spacing: 0.06em; text-transform: uppercase; }
  .text-art {
    position: absolute; inset: 0; display: flex; align-items: flex-end; padding: 14px;
    background: linear-gradient(135deg, hsl(var(--h) 55% 42%), hsl(calc(var(--h) + 40) 60% 18%));
  }
  .text-art span { font-size: clamp(16px, 1.4vw, 24px); font-weight: 700; line-height: 1.15; text-shadow: 0 2px 8px rgba(0,0,0,.4); }
</style>
