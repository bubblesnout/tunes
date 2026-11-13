<script lang="ts">
  // Big Picture's fast-scroll indicator: the screen dims and the current letter shows large.
  import { fastScroll } from "../fastscroll.svelte";
</script>

<div class="overlay" class:show={fastScroll.visible} aria-hidden="true">
  <span>{fastScroll.letter}</span>
</div>

<style>
  .overlay {
    position: fixed; inset: 0; z-index: 45; pointer-events: none;
    display: grid; place-items: center;
    background: rgba(0, 0, 0, 0.72);
    opacity: 0; transition: opacity 180ms var(--ease-out);
  }
  .overlay.show { opacity: 1; transition-duration: 90ms; }
  span {
    font-size: clamp(120px, 26vh, 300px); font-weight: 700; line-height: 1; color: #fff;
    text-shadow: 0 8px 40px rgba(0, 0, 0, 0.6);
    text-box: trim-both cap alphabetic;
  }
</style>
