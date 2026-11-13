<script lang="ts">
  import { api, isTauri, on, type HomeSection, type PadStatus, type SteamStatus } from "../api";
  import { app, loadHome, toast } from "../app.svelte";
  import {
    ACTIONS, BINDINGS_VERSION, DEFAULT_BINDINGS, captureNext, getBindings, inputLabel, setBindings, type Action, type Bindings,
  } from "../input";
  import { focusDefault } from "../nav";
  import Icon from "../components/Icon.svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { onDestroy, onMount, tick } from "svelte";

  let { tab = "home" }: { tab?: string } = $props();
  let current = $state("home");
  $effect(() => {
    current = tab;
  });

  const TABS = [
    { id: "home", label: "Home Screen", icon: "home" },
    { id: "controls", label: "Controller", icon: "gamepad" },
    { id: "general", label: "General", icon: "settings" },
    { id: "steam", label: "Steam", icon: "library" },
    { id: "account", label: "Account", icon: "user" },
  ];

  const LABELS: Record<string, { name: string; desc: string }> = {
    resume: { name: "Continue Listening", desc: "Resume your last queue" },
    recentlyAdded: { name: "Recently Added", desc: "Newest albums on the server" },
    recentlyPlayed: { name: "Recently Played", desc: "Albums you've played lately" },
    mostPlayed: { name: "Most Played", desc: "Your most frequently played albums" },
    starred: { name: "Favourites", desc: "Albums you've starred" },
    random: { name: "Discover", desc: "A random selection of albums" },
    highestRated: { name: "Top Rated", desc: "Your highest rated albums" },
    playlists: { name: "Playlists", desc: "Playlists on your server" },
  };

  let pad = $state<PadStatus | null>(null);
  onMount(() => {
    app.backdrop = null;
    tick().then(focusDefault);
    api.gamepadStatus().then((s) => (pad = s));
    const un = on<PadStatus>("pad-status", (p) => (pad = p));
    return () => un.then((f) => f());
  });
  onDestroy(() => captureNext(null));

  // ---- Home sections -----------------------------------------------------------------
  let sections = $state<HomeSection[]>(structuredClone($state.snapshot(app.config?.homeSections ?? [])));

  async function saveSections() {
    app.config = await api.saveSettings({ homeSections: $state.snapshot(sections) });
    loadHome();
  }

  function toggleSection(i: number) {
    sections[i].enabled = !sections[i].enabled;
    saveSections();
  }

  async function moveSection(i: number, delta: number, e: MouseEvent) {
    const j = i + delta;
    if (j < 0 || j >= sections.length) return;
    [sections[i], sections[j]] = [sections[j], sections[i]];
    saveSections();
    const which = (e.currentTarget as HTMLElement).dataset.move;
    await tick();
    const target = document.querySelector<HTMLButtonElement>(`[data-row="${j}"] [data-move="${which}"]`);
    (target && !target.disabled ? target : document.querySelector<HTMLElement>(`[data-row="${j}"] [data-nav]`))?.focus();
  }

  // ---- Controls ----------------------------------------------------------------------
  let binds = $state<Bindings>(getBindings());
  let capturing = $state<{ action: Action; device: "Pad" | "Key" } | null>(null);
  let captureTimer: ReturnType<typeof setTimeout> | undefined;

  function startCapture(action: Action, device: "Pad" | "Key") {
    capturing = { action, device };
    clearTimeout(captureTimer);
    captureTimer = setTimeout(() => {
      capturing = null;
      captureNext(null);
    }, 6000);
    setTimeout(() => {
      captureNext((input) => {
        clearTimeout(captureTimer);
        const c = capturing;
        capturing = null;
        if (!c) return;
        if (!input.startsWith(c.device + ":")) {
          toast(c.device === "Pad" ? "Press a gamepad button" : "Press a keyboard key");
          return;
        }
        for (const a of Object.keys(binds) as Action[]) binds[a] = binds[a].filter((x) => x !== input);
        binds[c.action] = [...binds[c.action].filter((x) => !x.startsWith(c.device + ":")), input];
        persistBindings();
      });
    }, 150);
  }

  async function persistBindings() {
    const saved = { ...$state.snapshot(binds), _v: BINDINGS_VERSION };
    setBindings(saved);
    app.config = await api.saveSettings({ bindings: saved as never });
  }

  async function resetBindings() {
    binds = structuredClone(DEFAULT_BINDINGS);
    await persistBindings();
    toast("Controls reset to defaults");
  }

  // ---- Steam -------------------------------------------------------------------------
  let steam = $state<SteamStatus | null>(null);
  let steamBusy = $state(false);
  let needsRestart = $state(false);

  $effect(() => {
    if (current === "steam" && !steam) api.steamStatus().then((s) => (steam = s));
  });

  async function steamShortcut(add: boolean) {
    steamBusy = true;
    try {
      steam = await api.steamShortcut(add);
      needsRestart = steam.running;
      toast(add ? (steam.running ? "Added to Steam. Restart Steam to see it." : "Added to Steam") : "Removed from Steam");
    } catch (e) {
      toast(String(e));
    } finally {
      steamBusy = false;
    }
  }

  async function toggleOverlay() {
    app.config = await api.saveSettings({ steamOverlay: !app.config?.steamOverlay });
    toast("Takes effect the next time Tunes starts");
  }

  async function restartSteam() {
    steamBusy = true;
    toast("Restarting Steam…");
    try {
      await api.steamRestart();
      needsRestart = false;
      steam = await api.steamStatus();
    } catch (e) {
      toast(String(e));
    } finally {
      steamBusy = false;
    }
  }

  // ---- General -----------------------------------------------------------------------
  async function toggleFullscreen() {
    if (!isTauri) return;
    const w = getCurrentWindow();
    const fs = !(await w.isFullscreen());
    await w.setFullscreen(fs);
    app.config = await api.saveSettings({ fullscreen: fs });
  }

  async function signOut() {
    app.config = await api.logout();
    app.signedIn = false;
    app.home = { albums: {}, playlists: [] };
  }
</script>

<div class="settings page">
  <nav class="rail" data-nav-group>
    {#each TABS as t}
      <button class:active={current === t.id} data-nav data-nav-default={current === t.id ? "" : undefined}
        onclick={() => (current = t.id)} onfocus={() => (current = t.id)}>
        <Icon name={t.icon} size={18} /> {t.label}
      </button>
    {/each}
  </nav>

  <section class="content" data-nav-group>
    {#if current === "home"}
      <h2>Home Screen</h2>
      <p class="desc">Choose which shelves appear on Home and in what order. The first shelf shows a large featured tile.</p>
      <div class="group label-caps">Shelves</div>
      <ul>
        {#each sections as s, i (s.id)}
          <li class="row" data-row={i}>
            <button class="row-main" data-nav onclick={() => toggleSection(i)}>
              <span class="text"><span class="name">{LABELS[s.id]?.name ?? s.id}</span><span class="sub">{LABELS[s.id]?.desc ?? ""}</span></span>
              <span class="switch" class:on={s.enabled}><span></span></span>
            </button>
            <button class="btn icon" data-nav data-move="up" disabled={i === 0} onclick={(e) => moveSection(i, -1, e)} title="Move up">▲</button>
            <button class="btn icon" data-nav data-move="down" disabled={i === sections.length - 1} onclick={(e) => moveSection(i, 1, e)} title="Move down">▼</button>
          </li>
        {/each}
      </ul>
    {:else if current === "controls"}
      <h2>Controller</h2>
      <p class="desc">Select a binding, then press the new button or key.</p>
      <div class="group label-caps">Status</div>
      <div class="status-box">
        <div class="kv"><span>Controller</span><span>{#if pad?.pads.length}<span class="ok">●</span> {pad.pads.join(", ")}{:else}<span class="bad">●</span> Not detected{/if}</span></div>
        <div class="kv"><span>Input API</span><span>{pad?.backend ?? "…"}</span></div>
        <div class="kv"><span>Last button</span><span>{pad?.lastInput ?? "None yet"}</span></div>
      </div>
      <div class="group label-caps">Bindings</div>
      <ul>
        {#each ACTIONS as a (a.id)}
          <li class="row bind">
            <span class="bind-name">{a.label}</span>
            {#each ["Pad", "Key"] as const as dev}
              {@const inputs = binds[a.id].filter((x) => x.startsWith(dev + ":"))}
              <button class="btn bind-btn" data-nav class:capturing={capturing?.action === a.id && capturing.device === dev}
                onclick={() => startCapture(a.id, dev)}>
                {#if capturing?.action === a.id && capturing.device === dev}
                  Press {dev === "Pad" ? "a button" : "a key"}…
                {:else}
                  <Icon name={dev === "Pad" ? "gamepad" : "keyboard"} size={16} />
                  <span class="ellipsis">{inputs.length ? inputs.map(inputLabel).join(" / ") : "—"}</span>
                {/if}
              </button>
            {/each}
          </li>
        {/each}
      </ul>
      <button class="btn" data-nav onclick={resetBindings}>Reset to defaults</button>
    {:else if current === "general"}
      <h2>General</h2>
      <ul>
        <li class="row">
          <button class="row-main" data-nav onclick={toggleFullscreen}>
            <span class="text"><span class="name">Fullscreen</span><span class="sub">Also toggled with F11</span></span>
            <span class="switch" class:on={app.config?.fullscreen}><span></span></span>
          </button>
        </li>
        <li class="row">
          <span class="row-main static"><span class="text"><span class="name">Library</span><span class="sub">Reload Home from the server</span></span></span>
          <button class="btn" data-nav onclick={() => loadHome(true)}>Refresh</button>
        </li>
      </ul>
      <p class="faint small">Tunes 0.1.2</p>
    {:else if current === "steam"}
      <h2>Steam</h2>
      <p class="desc">Add Tunes to your Steam library as a non-Steam app, with its own artwork, so you can launch it from Big Picture.</p>
      <div class="group label-caps">Status</div>
      <div class="status-box">
        <div class="kv"><span>Steam</span><span>{steam ? (steam.installed ? (steam.running ? "Installed, running" : "Installed") : "Not found") : "Checking…"}</span></div>
        {#if steam?.account}<div class="kv"><span>Account</span><span>{steam.account}</span></div>{/if}
        <div class="kv"><span>Tunes in library</span><span>{#if steam?.added}<span class="ok">●</span> Added{:else}Not added{/if}</span></div>
      </div>
      {#if steam?.installed}
        <ul>
          <li class="row">
            <span class="row-main static"><span class="text">
              <span class="name">{steam.added ? "Update Tunes in Steam" : "Add Tunes to Steam"}</span>
              <span class="sub">{steam.added ? "Refreshes the shortcut and artwork" : "Creates the shortcut and installs the library artwork"}</span>
            </span></span>
            <button class="btn" class:primary={!steam.added} data-nav disabled={steamBusy} onclick={() => steamShortcut(true)}>
              {steam.added ? "Update" : "Add to Steam"}
            </button>
          </li>
          {#if steam.added}
            <li class="row">
              <span class="row-main static"><span class="text"><span class="name">Remove from Steam</span><span class="sub">Deletes the shortcut and its artwork</span></span></span>
              <button class="btn" data-nav disabled={steamBusy} onclick={() => steamShortcut(false)}>Remove</button>
            </li>
          {/if}
          <li class="row">
            <button class="row-main" data-nav onclick={toggleOverlay}>
              <span class="text">
                <span class="name">Steam overlay support</span>
                <span class="sub">Lets the Steam overlay appear over Tunes when it's launched from Steam. Uses a little more power while Tunes is on screen. Takes effect the next time Tunes starts.{app.launch.overlayCompat ? " Active now." : ""}</span>
              </span>
              <span class="switch" class:on={app.config?.steamOverlay}><span></span></span>
            </button>
          </li>
          {#if needsRestart}
            <li class="row notice">
              <span class="row-main static"><span class="text">
                <span class="name">Restart Steam to apply</span>
                <span class="sub">Steam only reads shortcuts when it starts. Steam will close and reopen.</span>
              </span></span>
              <button class="btn" data-nav disabled={steamBusy} onclick={restartSteam}>Restart Steam</button>
            </li>
          {/if}
        </ul>
        <p class="desc">Tip: in Steam, open Tunes' controller settings and choose the <strong>Gamepad</strong> layout (or turn Steam Input off) so Tunes receives your controller directly.</p>
      {:else if steam}
        <p class="desc">Steam wasn't found on this device. Install Steam and sign in once, then come back here.</p>
      {/if}
    {:else}
      <h2>Account</h2>
      <div class="group label-caps">Server</div>
      <div class="status-box">
        <div class="kv"><span>Server</span><span>{app.config?.serverUrl}</span></div>
        <div class="kv"><span>User</span><span>{app.config?.username}</span></div>
        <div class="kv"><span>Status</span><span>{app.offline ? "Offline" : "Connected"}</span></div>
      </div>
      <ul>
        <li class="row">
          <span class="row-main static"><span class="text"><span class="name">Sign out</span><span class="sub">Removes the saved password from this device</span></span></span>
          <button class="btn" data-nav onclick={signOut}>Sign out</button>
        </li>
      </ul>
    {/if}
  </section>
</div>

<style>
  .settings { display: grid; grid-template-columns: clamp(220px, 19vw, 340px) minmax(0, 1fr); min-height: 100%; padding-left: calc(var(--gutter) * 2.2); }
  .rail {
    background: var(--surface-2); padding: 18px 0; display: flex; flex-direction: column; gap: 1px;
    position: sticky; top: 0; align-self: start; min-height: calc(100vh - var(--topbar-h) - var(--footer-h));
  }
  .rail button {
    display: flex; align-items: center; gap: 14px; padding: 11px 22px; text-align: left; font-size: 15px; color: #b8bcbf;
    transition: background var(--t-focus) var(--ease-out), color var(--t-focus);
  }
  .rail button.active { background: rgba(61, 68, 80, 0.6); color: var(--text); }
  .rail button:focus { background: var(--control); color: var(--text); }
  .content { padding: 22px clamp(24px, 3vw, 56px) 60px; max-width: 980px; }
  h2 { margin: 0 0 6px; font-size: 22px; font-weight: 600; }
  .desc { margin: 0 0 8px; color: var(--text-dim); font-size: 14px; }
  .group { margin: 24px 0 8px; }
  ul { list-style: none; margin: 0 0 22px; padding: 0; display: flex; flex-direction: column; gap: 2px; }
  .row { display: flex; gap: 6px; align-items: center; }
  .row > .btn.icon { min-height: 36px; width: 40px; font-size: 12px; }
  .row-main {
    flex: 1; display: flex; align-items: center; gap: 18px; padding: 12px 16px; border-radius: var(--radius-sm); text-align: left;
    transition: background var(--t-focus) var(--ease-out);
  }
  .row-main:not(.static):focus { background: var(--control); }
  .text { display: flex; flex-direction: column; gap: 2px; flex: 1; min-width: 0; }
  .name { font-size: 15px; color: var(--text); }
  .sub { font-size: 13px; color: var(--text-dim); }
  .small { font-size: 13px; }
  .switch { width: 44px; height: 22px; border-radius: 11px; background: var(--control); position: relative; flex-shrink: 0; transition: background var(--t-focus); }
  .switch span { position: absolute; top: 3px; left: 3px; width: 16px; height: 16px; border-radius: 50%; background: #8b929a; transition: translate var(--t-focus) var(--ease-out), background var(--t-focus); }
  .switch.on { background: var(--accent); }
  .switch.on span { translate: 22px 0; background: #fff; }
  .row-main:focus .switch:not(.on) { background: #59606b; }
  .status-box { background: rgba(35, 38, 46, 0.6); border-radius: var(--radius-sm); padding: 6px 16px; }
  .kv { display: flex; justify-content: space-between; gap: 20px; padding: 8px 0; font-size: 14px; color: var(--text-2); }
  .kv > span:first-child { color: var(--text-dim); }
  .ok { color: var(--green); }
  .bad { color: var(--danger); }
  .notice .row-main { border-left: 3px solid var(--accent); border-radius: 0; }
  .desc strong { color: var(--text-2); font-weight: 600; }
  .bind { align-items: center; }
  .bind-name { flex: 1; padding-left: 4px; font-size: 15px; color: var(--text-2); }
  .bind-btn { width: 210px; justify-content: flex-start; min-height: 40px; padding: 0 14px; font-size: 14px; }
  .bind-btn.capturing { background: var(--accent); color: #fff; animation: pulse 1s infinite; }
  @keyframes pulse { 50% { opacity: .7; } }
</style>
