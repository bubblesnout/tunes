<script lang="ts">
  // Pop-down volume slider under the top-bar volume icon. Left/right (handled in App)
  // or mouse drag adjusts in-app volume; it hides after a few seconds without input.
  import { api } from "../api";
  import { app } from "../app.svelte";
  import Icon from "./Icon.svelte";

  const HIDE_AFTER = 3000;
  const v = $derived(app.player?.volume ?? 0);

  $effect(() => {
    if (!app.volumePopup) return;
    const t = setTimeout(() => (app.volumePopup = 0), HIDE_AFTER);
    return () => clearTimeout(t);
  });

  // Clicking anywhere outside the volume control closes the slider.
  $effect(() => {
    if (!app.volumePopup) return;
    const onDown = (e: PointerEvent) => {
      if (!(e.target as HTMLElement).closest?.(".vol-wrap")) app.volumePopup = 0;
    };
    window.addEventListener("pointerdown", onDown, true);
    return () => window.removeEventListener("pointerdown", onDown, true);
  });

  let track: HTMLDivElement | undefined = $state();
  let dragging = false;

  function setFromPointer(e: PointerEvent) {
    if (!track) return;
    const r = track.getBoundingClientRect();
    const val = Math.min(1, Math.max(0, (e.clientX - r.left) / r.width));
    app.volumePopup = Date.now();
    if (app.player) app.player.volume = Math.round(val * 100) / 100;
    api.player("volume", val);
  }
</script>

{#if app.volumePopup}
  <div class="pop" role="presentation" onpointerdown={(e) => e.stopPropagation()} onpointermove={() => (app.volumePopup = Date.now())}>
    <Icon name={v === 0 ? "mute" : "volume"} size={20} />
    <div class="track" bind:this={track} role="slider" tabindex="-1" aria-label="Volume" aria-valuemin={0} aria-valuemax={100} aria-valuenow={Math.round(v * 100)}
      onpointerdown={(e) => { dragging = true; try { track?.setPointerCapture(e.pointerId); } catch { /* not capturable */ } setFromPointer(e); }}
      onpointermove={(e) => dragging && setFromPointer(e)}
      onpointerup={() => (dragging = false)}
      onpointercancel={() => (dragging = false)}>
      <div class="rail"><div class="fill" style:width="{v * 100}%"></div></div>
      <div class="knob" style:left="{v * 100}%"></div>
    </div>
    <span class="num">{Math.round(v * 100)}</span>
  </div>
{/if}

<style>
  .pop {
    position: absolute; top: calc(100% + 6px); right: 0; z-index: 20;
    display: flex; align-items: center; gap: 14px; padding: 14px 18px; border-radius: var(--radius);
    background: var(--surface); box-shadow: 0 16px 48px rgba(0, 0, 0, 0.65); color: var(--text-2);
    animation: drop 160ms var(--ease-out);
  }
  .track { position: relative; width: 240px; height: 24px; display: flex; align-items: center; cursor: pointer; touch-action: none; }
  .rail { width: 100%; height: 4px; border-radius: 2px; background: var(--control); overflow: hidden; }
  .fill { height: 100%; background: var(--accent); }
  .knob {
    position: absolute; top: 50%; width: 16px; height: 16px; border-radius: 50%; background: #fff;
    translate: -50% -50%; box-shadow: 0 2px 6px rgba(0, 0, 0, 0.5);
  }
  .num { width: 30px; text-align: right; font-weight: 500; font-variant-numeric: tabular-nums; }
  @keyframes drop { from { opacity: 0; transform: translateY(-6px); } }
</style>
