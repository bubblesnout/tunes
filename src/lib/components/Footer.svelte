<script lang="ts">
  // Big Picture footer: MENU on the left, contextual uppercase button hints on the right.
  // Hints are clickable for mouse users and trigger the same action as the button.
  import { app } from "../app.svelte";
  import { hintFor, inputLabel, type Action } from "../input";

  let { hints, onaction }: { hints: { action: Action; label: string }[]; onaction: (a: Action) => void } = $props();

  function resolve(list: { action: Action; label: string }[]) {
    return list
      .map((h) => ({ ...h, input: hintFor(h.action, app.device) }))
      .filter((h): h is typeof h & { input: string } => !!h.input);
  }
  const right = $derived(resolve(hints));
  const left = $derived(app.signedIn ? resolve([{ action: "menu", label: "Menu" }]) : []);

  function cls(input: string) {
    const n = input.split(":")[1];
    if (["A", "B", "X", "Y"].includes(n)) return `glyph face ${n}`;
    if (n === "Menu" || n === "View") return "glyph round";
    return "glyph key";
  }
</script>

<footer>
  <div class="side">
    {#each left as h (h.action)}
      <button class="hint" tabindex="-1" onpointerdown={(e) => e.preventDefault()} onclick={() => onaction(h.action)}>
        <span class={cls(h.input)}>{inputLabel(h.input)}</span>{h.label}
      </button>
    {/each}
  </div>
  <div class="side end">
    {#each right as h (h.action)}
      <!-- preventDefault on pointerdown keeps focus on the current item, so "Select" acts on it -->
      <button class="hint" tabindex="-1" onpointerdown={(e) => e.preventDefault()} onclick={() => onaction(h.action)}>
        <span class={cls(h.input)}>{inputLabel(h.input)}</span>{h.label}
      </button>
    {/each}
  </div>
</footer>

<style>
  footer {
    height: var(--footer-h); flex-shrink: 0; display: flex; align-items: center; justify-content: space-between;
    padding: 0 calc(var(--gutter) - 10px); position: relative; z-index: 35;
    background: linear-gradient(180deg, rgba(14, 17, 22, 0.86), rgba(10, 12, 15, 0.96));
    box-shadow: 0 -1px 0 rgba(255, 255, 255, 0.04);
  }
  .side { display: flex; align-items: center; gap: 4px; }
  .hint {
    display: flex; align-items: center; gap: 8px; padding: 6px 10px; border-radius: var(--radius-sm);
    font-size: 12px; font-weight: 700; letter-spacing: 0.07em; text-transform: uppercase; color: var(--text-2);
    transition: background var(--t-focus) var(--ease-out);
  }
  .hint:hover { background: rgba(255, 255, 255, 0.08); }
  .hint:active { background: rgba(255, 255, 255, 0.14); }
  .glyph {
    height: 22px; min-width: 22px; display: grid; place-items: center; font-weight: 800; letter-spacing: 0;
    background: #3a3f47; color: #e8eaec;
  }
  .face { border-radius: 50%; font-size: 12px; }
  .face.A { color: var(--btn-a); } .face.B { color: var(--btn-b); }
  .face.X { color: var(--btn-x); } .face.Y { color: var(--btn-y); }
  .round { border-radius: 11px; padding: 0 7px; font-size: 13px; }
  .key { border-radius: var(--radius); padding: 0 6px; font-size: 11px; }
</style>
