<script lang="ts">
  import { api } from "./lib/api";
  import { app, back, init, navigate, route } from "./lib/app.svelte";
  import { initInput, onAction, onDeviceChange, type Action } from "./lib/input";
  import { current, ensureFocus, move, page } from "./lib/nav";
  import TopBar from "./lib/components/TopBar.svelte";
  import Footer from "./lib/components/Footer.svelte";
  import VolumeOsd from "./lib/components/VolumeOsd.svelte";
  import Menu from "./lib/components/Menu.svelte";
  import Toast from "./lib/components/Toast.svelte";
  import Drawer from "./lib/components/Drawer.svelte";
  import Backdrop from "./lib/components/Backdrop.svelte";
  import Login from "./lib/screens/Login.svelte";
  import Home from "./lib/screens/Home.svelte";
  import Collection from "./lib/screens/Collection.svelte";
  import NowPlaying from "./lib/screens/NowPlaying.svelte";
  import Settings from "./lib/screens/Settings.svelte";
  import Library from "./lib/screens/Library.svelte";
  import Artist from "./lib/screens/Artist.svelte";
  import Favourites from "./lib/screens/Favourites.svelte";
  import LetterOverlay from "./lib/components/LetterOverlay.svelte";
  import { verticalMove } from "./lib/fastscroll.svelte";
  import { onMount, tick } from "svelte";

  const VOLUME_STEP = 0.05;

  onMount(async () => {
    await init();
    await initInput();
    onDeviceChange((d) => (app.device = d));
    onAction(handle);
  });

  function handle(action: Action, repeat = false) {
    // While the volume slider is open, left/right adjust it and anything else dismisses it.
    if (app.volumePopup) {
      if (action === "left" || action === "right" || action === "volDown" || action === "volUp") {
        app.volumePopup = Date.now();
        api.player("volumeBy", action === "left" || action === "volDown" ? -VOLUME_STEP : VOLUME_STEP);
        return;
      }
      app.volumePopup = 0;
      if (action === "select" || action === "back" || action === "up" || action === "down") return;
    }
    switch (action) {
      case "up":
      case "down":
      case "left":
      case "right":
        if (verticalMove(action, repeat)) return;
        // Pushing left from the far-left edge opens the main menu, as in Big Picture.
        if (!move(action) && action === "left" && app.signedIn && !app.menu && !app.drawer) app.drawer = true;
        else if (action === "right" && app.drawer && !move(action)) app.drawer = false;
        return;
      case "select": {
        const el = current();
        if (el instanceof HTMLInputElement && el.form) {
          const fields = Array.from(el.form.querySelectorAll<HTMLInputElement>("input"));
          const next = fields[fields.indexOf(el) + 1];
          if (next && !next.value) next.focus();
          else el.form.requestSubmit();
          return;
        }
        el?.click();
        return;
      }
      case "back":
        if (back()) tick().then(ensureFocus);
        // At the root of a section there's nowhere to go back to: open the main menu, as Steam does.
        else if (app.signedIn) app.drawer = true;
        return;
      case "options": {
        const el = current();
        if (el) el.dispatchEvent(new CustomEvent("navoptions"));
        return;
      }
      case "pageUp":
        page(-1);
        return;
      case "pageDown":
        page(1);
        return;
    }
    if (!app.signedIn) return;
    switch (action) {
      case "menu":
        app.menu = null;
        app.drawer = !app.drawer;
        if (!app.drawer) tick().then(ensureFocus);
        break;
      case "playPause":
        api.player("toggle");
        break;
      case "prevTrack":
        api.player("prev");
        break;
      case "nextTrack":
        api.player("next");
        break;
      case "volDown":
      case "volUp":
        api.player("volumeBy", action === "volDown" ? -VOLUME_STEP : VOLUME_STEP);
        break;
      case "nowPlaying":
        app.menu = null;
        if (route().name === "nowPlaying") back();
        else navigate({ name: "nowPlaying" });
        break;
    }
  }

  const r = $derived(route());
  const hints = $derived.by(() => {
    const h: { action: Action; label: string }[] = [];
    if (app.signedIn && !app.menu && !app.drawer && !["nowPlaying", "settings"].includes(r.name)) {
      h.push({ action: "options", label: "Options" });
    }
    h.push({ action: "select", label: "Select" });
    if (app.menu || app.drawer || app.stack.length > 1) h.push({ action: "back", label: "Back" });
    else if (app.signedIn) h.push({ action: "back", label: "Menu" });
    return h;
  });

  let scroller: HTMLElement | undefined = $state();
  $effect(() => {
    r;
    scroller?.scrollTo({ top: 0 });
  });
</script>

{#if app.ready}
  <div class="shell" class:pushed={app.drawer}>
    {#if app.signedIn}
      <Backdrop />
      <TopBar />
      <main bind:this={scroller} data-scroll-main>
        {#key JSON.stringify(r)}
          {#if r.name === "home"}
            <Home />
          {:else if r.name === "album"}
            <Collection kind="album" id={r.id} />
          {:else if r.name === "playlist"}
            <Collection kind="playlist" id={r.id} />
          {:else if r.name === "artist"}
            <Artist id={r.id} />
          {:else if r.name === "genre"}
            <Library kind="genre" genre={r.id} />
          {:else if r.name === "albumArtists" || r.name === "artists" || r.name === "albums" || r.name === "playlists" || r.name === "genres"}
            <Library kind={r.name} />
          {:else if r.name === "favourites"}
            <Favourites />
          {:else if r.name === "nowPlaying"}
            <NowPlaying />
          {:else if r.name === "settings"}
            <Settings tab={r.tab} />
          {/if}
        {/key}
      </main>
    {:else}
      <main class="login-main"><Login /></main>
    {/if}
  </div>
  {#if app.signedIn}<Drawer />{/if}
  <Footer {hints} onaction={handle} />
  <VolumeOsd />
  <LetterOverlay />
  <Menu />
  <Toast />
{/if}

<style>
  :global(#app) { display: flex; flex-direction: column; }
  .shell {
    position: relative; flex: 1; min-height: 0; display: flex; flex-direction: column;
    transition: transform var(--t-drawer) var(--ease-out), filter var(--t-drawer) var(--ease-out);
  }
  /* Content shifts slightly and softens when the main menu slides out */
  .shell.pushed { transform: translateX(calc(var(--drawer-w) * 0.12)); filter: brightness(0.7); }
  .shell > :global(.top) { position: absolute; top: 0; left: 0; right: 0; }
  main { flex: 1; overflow-y: auto; overflow-x: hidden; scroll-behavior: smooth; padding-top: var(--topbar-h); }
  .login-main { padding-top: 0; }
</style>
