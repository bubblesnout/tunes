<script lang="ts">
  // The Steam overlay only draws when the app presents a new frame, and Chromium only
  // presents when something changes. When Steam launched Tunes (and overlay support is on),
  // this keeps a 1px compositor animation running so frames keep flowing and the overlay
  // can appear and update. It stops while the window is hidden or minimised, so it costs
  // nothing when a game is in front.
  let visible = $state(!document.hidden);
  $effect(() => {
    const onVis = () => (visible = !document.hidden);
    document.addEventListener("visibilitychange", onVis);
    return () => document.removeEventListener("visibilitychange", onVis);
  });
</script>

{#if visible}
  <div class="keepalive" aria-hidden="true"></div>
{/if}

<style>
  .keepalive {
    position: fixed; left: 0; bottom: 0; width: 1px; height: 1px; z-index: 9999; pointer-events: none;
    background: #0e1116;
    /* Needs a real (if imperceptible) change every frame so Chromium keeps presenting. */
    animation: tick 1s linear infinite;
  }
  @keyframes tick {
    from { opacity: 0.98; }
    to { opacity: 1; }
  }
</style>
