<script lang="ts">
  // The bar under a page's hero: big green Play, stats, then shuffle / play next / add to queue.
  // `songs` may be async so pages like Artist only fetch tracks when a button is pressed.
  import type { Song } from "../api";
  import { toast } from "../app.svelte";
  import { playSongs, queueSongs } from "../actions";
  import Icon from "./Icon.svelte";

  let {
    title, stats = [], disabled = false, songs,
  }: { title: string; stats?: { label: string; value: string }[]; disabled?: boolean; songs: () => Song[] | Promise<Song[]> } = $props();

  let busy = $state(false);
  async function run(f: (s: Song[]) => unknown) {
    busy = true;
    try {
      await f(await songs());
    } catch (e) {
      toast(String(e));
    } finally {
      busy = false;
    }
  }
</script>

<div class="actionbar" data-nav-group>
  <div class="actions">
    <button class="btn play playbig" data-nav data-nav-default data-nav-entry disabled={disabled || busy} onclick={() => run((s) => playSongs(s))}>
      <Icon name="play" /> Play
    </button>
  </div>
  <div class="stats">
    {#each stats as s}
      <div class="stat"><div class="label-caps">{s.label}</div><div class="value">{s.value}</div></div>
    {/each}
  </div>
  <div class="actions end">
    <button class="btn icon" data-nav disabled={disabled || busy} onclick={() => run((s) => playSongs(s, 0, true))} title="Shuffle"><Icon name="shuffle" /></button>
    <button class="btn icon" data-nav disabled={disabled || busy} onclick={() => run((s) => queueSongs(s, true, title))} title="Play next"><Icon name="queue" /></button>
    <button class="btn icon" data-nav disabled={disabled || busy} onclick={() => run((s) => queueSongs(s, false, title))} title="Add to queue"><Icon name="plus" /></button>
  </div>
</div>

<style>
  .actionbar {
    display: flex; align-items: center; gap: 36px; padding: 14px var(--gutter);
    background: rgba(14, 17, 22, 0.85); border-bottom: 1px solid rgba(255,255,255,.04);
  }
  .actions { display: flex; gap: 8px; }
  .actions.end { margin-left: auto; }
  .playbig { min-width: 200px; justify-content: flex-start; font-size: 17px; }
  .stats { display: flex; gap: 32px; }
  .stat .value { font-size: 15px; color: var(--text-2); margin-top: 3px; }
</style>
