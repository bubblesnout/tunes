<script lang="ts">
  import type { Snippet } from "svelte";
  let { title, children, compact = false }: { title: string; children: Snippet; compact?: boolean } = $props();
</script>

<section class="shelf" class:compact>
  <h2>{title}</h2>
  <div class="row" data-nav-group>
    {@render children()}
  </div>
</section>

<style>
  .shelf { margin-bottom: -12px; }
  h2 { margin: 0 var(--gutter) 0; font-size: 19px; font-weight: 600; color: var(--text-2); }
  /* Compact shelves have no captions under tiles, so need less bottom room */
  .compact .row { padding-bottom: 30px; }
  .row {
    display: flex; gap: var(--tile-gap); overflow-x: auto; overflow-y: visible;
    /* Vertical padding leaves room for the scaled focus state and caption */
    padding: calc(var(--tile) * 0.07 + 4px) var(--gutter) 66px;
    scroll-padding: 0 calc(var(--gutter) + var(--tile) * 0.06);
    scroll-behavior: smooth;
  }
</style>
