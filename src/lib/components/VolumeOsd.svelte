<script lang="ts">
  import { app } from "../app.svelte";
  import Icon from "./Icon.svelte";
  let visible = $state(false);
  $effect(() => {
    if (!app.volumeOsd) return;
    visible = true;
    const t = setTimeout(() => (visible = false), 1400);
    return () => clearTimeout(t);
  });
  const v = $derived(app.player?.volume ?? 0);
</script>

<div class="osd" class:visible={visible && !app.volumePopup}>
  <Icon name={v === 0 ? "mute" : "volume"} size={22} />
  <div class="bar"><div class="fill" style:width="{v * 100}%"></div></div>
  <span class="num">{Math.round(v * 100)}</span>
</div>

<style>
  .osd {
    position: fixed; top: 14px; left: 50%; translate: -50% -16px; opacity: 0; pointer-events: none; z-index: 50;
    display: flex; align-items: center; gap: 14px; padding: 12px 18px; border-radius: var(--radius);
    background: var(--surface); box-shadow: 0 12px 40px rgba(0,0,0,.6);
    transition: opacity 150ms var(--ease-out), translate 150ms var(--ease-out);
  }
  .osd.visible { opacity: 1; translate: -50% 0; }
  .bar { width: 220px; height: 4px; border-radius: 2px; background: var(--control); overflow: hidden; }
  .fill { height: 100%; background: var(--accent); transition: width 80ms linear; }
  .num { width: 30px; text-align: right; font-weight: 600; font-variant-numeric: tabular-nums; color: var(--text-2); }
</style>
