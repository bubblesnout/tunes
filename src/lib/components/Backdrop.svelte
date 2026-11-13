<script lang="ts">
  // Faded art of the focused item behind the top of the screen, cross-fading between
  // items — Big Picture's home screen does this with the focused game's hero art.
  import { app } from "../app.svelte";
  import { coverUrl } from "../api";

  let layers = $state<{ id: string; key: number }[]>([]);
  let n = 0;
  let timer: ReturnType<typeof setTimeout> | undefined;

  $effect(() => {
    const id = app.backdrop;
    clearTimeout(timer);
    // Small debounce so scrolling quickly along a shelf doesn't thrash images.
    timer = setTimeout(() => {
      if (!id) {
        layers = [];
        return;
      }
      if (layers.at(-1)?.id === id) return;
      layers = [...layers.slice(-1), { id, key: ++n }];
    }, 120);
  });
</script>

<div class="backdrop" aria-hidden="true">
  {#each layers as l (l.key)}
    <div class="layer" style:background-image={`url("${coverUrl(l.id, 300)}")`}></div>
  {/each}
</div>

<style>
  .backdrop {
    position: fixed; inset: 0 0 auto 0; height: 70vh; z-index: -1; pointer-events: none; overflow: hidden;
    -webkit-mask-image: linear-gradient(180deg, rgba(0,0,0,.9) 0%, rgba(0,0,0,.5) 45%, transparent 100%);
    mask-image: linear-gradient(180deg, rgba(0,0,0,.9) 0%, rgba(0,0,0,.5) 45%, transparent 100%);
  }
  .layer {
    position: absolute; inset: -60px; background-size: cover; background-position: center 30%;
    filter: blur(40px) saturate(1.3) brightness(0.42);
    animation: in 600ms ease-out both;
  }
  @keyframes in { from { opacity: 0; transform: scale(1.04); } }
</style>
