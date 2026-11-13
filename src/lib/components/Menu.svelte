<script lang="ts">
  // Options popup (Big Picture's context menu).
  import { app } from "../app.svelte";
  import { focusDefault } from "../nav";
  import { tick } from "svelte";

  let previous: HTMLElement | null = null;
  $effect(() => {
    if (app.menu) {
      previous = document.activeElement as HTMLElement;
      tick().then(focusDefault);
    } else if (previous) {
      if (previous.isConnected) previous.focus({ preventScroll: true });
      previous = null;
    }
  });
</script>

{#if app.menu}
  <div class="scrim" role="presentation" onclick={() => (app.menu = null)}></div>
  <div class="menu" data-nav-scope>
    <div class="head ellipsis">{app.menu.title}</div>
    {#each app.menu.items as item, i}
      <button data-nav data-nav-default={i === 0 ? "" : undefined}
        onclick={async () => { app.menu = null; await item.run(); }}>{item.label}</button>
    {/each}
  </div>
{/if}

<style>
  .scrim { position: fixed; inset: 0; background: var(--scrim); z-index: 40; animation: fade var(--t-fade) var(--ease-out); }
  .menu {
    position: fixed; left: 50%; top: 50%; translate: -50% -50%; z-index: 41; width: min(440px, 90vw);
    padding: 8px 0; border-radius: var(--radius); background: var(--surface);
    box-shadow: 0 24px 80px rgba(0, 0, 0, 0.7); display: flex; flex-direction: column;
    animation: pop 180ms var(--ease-out);
  }
  .head { padding: 14px 22px 12px; font-size: 12px; font-weight: 700; letter-spacing: .08em; text-transform: uppercase; color: var(--text-dim); border-bottom: 1px solid rgba(255,255,255,.06); margin-bottom: 6px; }
  button { text-align: left; padding: 13px 22px; font-size: 16px; color: var(--text-2); transition: background var(--t-focus), color var(--t-focus); }
  button:focus { background: var(--control); color: var(--text); }
  @keyframes fade { from { opacity: 0; } }
  @keyframes pop { from { opacity: 0; transform: scale(0.96); } }
</style>
