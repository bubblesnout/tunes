<script lang="ts">
  import { app } from "../app.svelte";
  let visible = $state(false);
  $effect(() => {
    if (!app.toast) return;
    visible = true;
    const t = setTimeout(() => (visible = false), 3500);
    return () => clearTimeout(t);
  });
</script>

<div class="toast" class:visible>{app.toast?.text ?? ""}</div>

<style>
  .toast {
    position: fixed; bottom: calc(var(--footer-h) + 20px); left: 50%; translate: -50% 10px; opacity: 0; z-index: 60;
    padding: 14px 22px; border-radius: var(--radius); background: var(--surface); box-shadow: 0 10px 30px rgba(0,0,0,.5);
    font-size: 15px; pointer-events: none; transition: opacity 160ms, translate 160ms var(--ease-out); max-width: 70vw;
  }
  .toast.visible { opacity: 1; translate: -50% 0; }
</style>
