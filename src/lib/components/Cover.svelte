<script lang="ts">
  import { coverUrl } from "../api";
  let { id, size = 300, alt = "" }: { id?: string | null; size?: number; alt?: string } = $props();
  // Track load/fail per id so a fast (cached) load can never be reset afterwards.
  let loadedId = $state<string | null>(null);
  let failedId = $state<string | null>(null);
  const loaded = $derived(!!id && loadedId === id);
  const failed = $derived(!!id && failedId === id);

  function check(img: HTMLImageElement) {
    if (img.complete && img.naturalWidth > 0) loadedId = id ?? null;
  }
</script>

<div class="cover" class:loaded>
  {#if id && !failed}
    {#key id}
      <img src={coverUrl(id, size)} {alt} loading="lazy" decoding="async" draggable="false" use:check
        onload={() => (loadedId = id ?? null)} onerror={() => (failedId = id ?? null)} />
    {/key}
  {:else}
    <svg viewBox="0 0 24 24" fill="currentColor"><path d="M12 3v10.6A4 4 0 1 0 14 17V7h4V3h-6z" /></svg>
  {/if}
</div>

<style>
  .cover {
    position: relative; width: 100%; aspect-ratio: 1; border-radius: inherit; overflow: hidden;
    background: linear-gradient(135deg, var(--surface-3), var(--surface-2));
    display: grid; place-items: center; color: var(--text-faint);
  }
  img { position: absolute; inset: 0; width: 100%; height: 100%; object-fit: cover; opacity: 0; transition: opacity 260ms ease-out; }
  .loaded img { opacity: 1; }
  svg { width: 34%; height: 34%; }
</style>
