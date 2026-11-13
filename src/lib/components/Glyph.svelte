<script lang="ts">
  // On-screen button glyph for an input id ("Pad:A", "Key:Enter"…).
  // Pad buttons are drawn as SVG so the symbol is geometrically centred whatever the font;
  // text labels (bumpers, triggers, keys) are trimmed to cap height so they centre too.
  import { inputLabel } from "../input";

  let { input }: { input: string } = $props();

  const kind = $derived(input.split(":")[0]);
  const name = $derived(input.split(":")[1] ?? "");

  // Stroked symbols in a 24×24 box, centred on (12, 12).
  const SYMBOLS: Record<string, string> = {
    A: "M8.2 17 12 7l3.8 10M9.6 13.4h4.8",
    B: "M8.8 7v10M8.8 7h3.4a2.45 2.45 0 0 1 0 4.9H8.8M8.8 11.9h3.9a2.55 2.55 0 0 1 0 5.1H8.8",
    X: "M8.6 7l6.8 10M15.4 7l-6.8 10",
    Y: "M8.4 7 12 12.2 15.6 7M12 12.2V17",
    Menu: "M7.5 8.5h9M7.5 12h9M7.5 15.5h9",
    View: "M7.5 7.5h6v6h-6zM10.5 13.5v3h6v-6h-3",
  };
  const face = $derived(kind === "Pad" && ["A", "B", "X", "Y"].includes(name));
  const symbol = $derived(kind === "Pad" ? SYMBOLS[name] : undefined);
</script>

{#if symbol}
  <span class="glyph round" class:face data-btn={name} aria-label={name}>
    <svg viewBox="0 0 24 24" aria-hidden="true"><path d={symbol} /></svg>
  </span>
{:else}
  <span class="glyph pill" class:key={kind === "Key"}><span class="txt">{inputLabel(input)}</span></span>
{/if}

<style>
  .glyph {
    display: inline-grid; place-items: center; flex-shrink: 0;
    height: var(--glyph, 28px); min-width: var(--glyph, 28px);
    background: #3a3f47; color: #e8eaec;
  }
  .round { width: var(--glyph, 28px); border-radius: 50%; }
  .round svg {
    width: 100%; height: 100%; display: block;
    fill: none; stroke: currentColor; stroke-width: 2.1; stroke-linecap: round; stroke-linejoin: round;
  }
  .face[data-btn="A"] { color: var(--btn-a); }
  .face[data-btn="B"] { color: var(--btn-b); }
  .face[data-btn="X"] { color: var(--btn-x); }
  .face[data-btn="Y"] { color: var(--btn-y); }
  .pill { border-radius: calc(var(--glyph, 28px) / 2); padding: 0 9px; }
  .pill.key { border-radius: var(--radius); }
  .txt {
    font-size: calc(var(--glyph, 28px) * 0.43); font-weight: 800; letter-spacing: 0.02em; line-height: 1;
    /* Trim the font's ascender/descender space so the caps sit exactly in the middle. */
    text-box: trim-both cap alphabetic;
  }
</style>
