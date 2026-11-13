<script lang="ts">
  // Slide-out main menu from the left, as in Steam Big Picture.
  import { api } from "../api";
  import { app, currentSong, navigate, route } from "../app.svelte";
  import { focusDefault } from "../nav";
  import Icon from "./Icon.svelte";
  import Cover from "./Cover.svelte";
  import { tick } from "svelte";

  const items = [
    { id: "home", label: "Home", icon: "home" },
    { id: "nowPlaying", label: "Now Playing", icon: "music" },
    { id: "settings", label: "Settings", icon: "settings" },
  ] as const;

  let previous: HTMLElement | null = null;
  $effect(() => {
    if (app.drawer) {
      previous = document.activeElement as HTMLElement;
      tick().then(focusDefault);
    } else if (previous) {
      if (previous.isConnected) previous.focus({ preventScroll: true });
      previous = null;
    }
  });

  const active = $derived(route().name);
  const song = $derived(currentSong());
  const initial = $derived((app.config?.username ?? "?").slice(0, 1).toUpperCase());
  const server = $derived((app.config?.serverUrl ?? "").replace(/^https?:\/\//, ""));
</script>

<div class="scrim" class:open={app.drawer} role="presentation" onclick={() => (app.drawer = false)}></div>
<aside class="drawer" class:open={app.drawer} data-nav-scope={app.drawer ? "" : undefined} inert={!app.drawer}>
  <div class="profile">
    <div class="avatar">{initial}</div>
    <div class="who">
      <div class="name ellipsis">{app.config?.username}</div>
      <div class="status ellipsis" class:offline={app.offline}>{app.offline ? "Offline" : "Online"} · {server}</div>
    </div>
  </div>

  <nav>
    {#each items as it}
      {@const isNp = it.id === "nowPlaying" && !!song}
      <button class="item" class:np-item={isNp} class:active={active === it.id || (it.id === "home" && (active === "album" || active === "playlist"))}
        data-nav data-nav-default={active === it.id ? "" : undefined}
        onclick={() => navigate({ name: it.id })}>
        {#if isNp && song}
          <span class="np-art"><Cover id={song.coverArt} size={96} /></span>
          <span class="np-text">
            <span class="np-label" class:playing={!app.player?.paused}>{app.player?.paused ? "Paused" : "Now Playing"}</span>
            <span class="np-title ellipsis">{song.title}</span>
            <span class="np-artist ellipsis">{song.artist ?? ""}</span>
          </span>
        {:else}
          <Icon name={it.icon} size={22} />
          <span>{it.label}</span>
        {/if}
      </button>
    {/each}
  </nav>

  <div class="spacer"></div>
  <button class="item power" data-nav onclick={() => api.quit()}>
    <Icon name="power" size={22} />
    <span>Quit Tunes</span>
  </button>
</aside>

<style>
  .scrim {
    position: fixed; inset: 0; z-index: 30; background: var(--scrim); opacity: 0; pointer-events: none;
    transition: opacity var(--t-drawer) var(--ease-out);
  }
  .scrim.open { opacity: 1; pointer-events: auto; }
  .drawer {
    position: fixed; top: 0; bottom: 0; left: 0; z-index: 31; width: var(--drawer-w);
    display: flex; flex-direction: column; padding: 28px 0 calc(var(--footer-h) + 18px);
    background: linear-gradient(180deg, #1c2026 0%, #15181d 100%);
    box-shadow: 12px 0 48px rgba(0, 0, 0, 0.6);
    transform: translateX(-104%);
    transition: transform var(--t-drawer) var(--ease-out);
  }
  .drawer.open { transform: none; }
  .profile { display: flex; align-items: center; gap: 14px; padding: 8px 28px 28px; }
  .avatar {
    width: 48px; height: 48px; border-radius: var(--radius); display: grid; place-items: center; flex-shrink: 0;
    background: linear-gradient(135deg, var(--accent), #2a4fd8); font-weight: 800; font-size: 20px;
  }
  .who { min-width: 0; }
  .name { font-size: 17px; font-weight: 600; }
  .status { font-size: 13px; color: var(--accent-2); margin-top: 2px; }
  .status.offline { color: var(--text-dim); }
  nav { display: flex; flex-direction: column; gap: 2px; }
  .item {
    display: flex; align-items: center; gap: 18px; padding: 14px 28px; text-align: left;
    font-size: 19px; font-weight: 500; color: #b8bcbf;
    transition: background var(--t-focus) var(--ease-out), color var(--t-focus);
  }
  .item.active { color: var(--text); }
  .item:focus { background: var(--control); color: var(--text); }
  .item:focus :global(svg) { color: var(--text); }
  .np-item { padding-top: 10px; padding-bottom: 10px; gap: 14px; }
  .np-art { width: 52px; height: 52px; flex-shrink: 0; border-radius: var(--radius-sm); overflow: hidden; box-shadow: 0 4px 12px rgba(0,0,0,.4); }
  .np-text { display: flex; flex-direction: column; min-width: 0; line-height: 1.25; }
  .np-label { font-size: 11px; font-weight: 700; letter-spacing: 0.08em; text-transform: uppercase; color: var(--text-dim); }
  .np-label.playing { color: var(--green); }
  .np-title { font-size: 16px; font-weight: 500; color: var(--text); }
  .np-artist { font-size: 13px; color: var(--text-dim); }
  .item:focus .np-artist { color: var(--text-2); }
  .spacer { flex: 1; }
  .power { font-size: 16px; }
</style>
