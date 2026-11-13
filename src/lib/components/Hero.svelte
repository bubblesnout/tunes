<script lang="ts">
  // Page banner like a Big Picture game page: blurred artwork behind, art + titles in front.
  import { coverUrl } from "../api";
  import Cover from "./Cover.svelte";

  let { cover, kind, title, subtitle = "" }: { cover?: string | null; kind: string; title: string; subtitle?: string } = $props();
</script>

<div class="hero">
  {#if cover}<div class="hero-bg" style:background-image={`url("${coverUrl(cover, 600)}")`}></div>{/if}
  <div class="hero-content">
    <div class="art"><Cover id={cover} size={600} alt={title} /></div>
    <div class="titles">
      <div class="label-caps kind">{kind}</div>
      <h1>{title}</h1>
      {#if subtitle}<div class="subtitle">{subtitle}</div>{/if}
    </div>
  </div>
</div>

<style>
  .hero { position: relative; height: clamp(260px, 42vh, 460px); overflow: hidden; margin-top: calc(var(--topbar-h) * -1); }
  .hero-bg {
    position: absolute; inset: -80px; background-size: cover; background-position: center 35%;
    filter: blur(36px) saturate(1.5) brightness(0.55); transform: scale(1.15);
  }
  .hero::after { content: ""; position: absolute; inset: 0; background: linear-gradient(180deg, rgba(0,0,0,.1) 40%, rgba(14,17,22,.85) 100%); }
  .hero-content { position: absolute; z-index: 1; left: var(--gutter); right: var(--gutter); bottom: 26px; display: flex; align-items: flex-end; gap: 28px; }
  .art { width: clamp(150px, 15vw, 240px); flex-shrink: 0; border-radius: var(--radius-sm); overflow: hidden; box-shadow: 0 16px 48px rgba(0,0,0,.6); }
  .titles { min-width: 0; padding-bottom: 6px; }
  .kind { color: var(--text-2); }
  h1 { margin: 6px 0 4px; font-size: clamp(28px, 3vw, 52px); line-height: 1.05; font-weight: 700; text-shadow: 0 2px 12px rgba(0,0,0,.5); }
  .subtitle { font-size: 18px; color: var(--text-2); }
</style>
