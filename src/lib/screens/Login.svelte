<script lang="ts">
  import { api } from "../api";
  import { app, loadHome } from "../app.svelte";
  import { focusDefault } from "../nav";
  import { onMount, tick } from "svelte";

  let server = $state(app.config?.serverUrl ?? "");
  let username = $state(app.config?.username ?? "");
  let password = $state("");
  let busy = $state(false);
  let error = $state("");

  onMount(() => tick().then(focusDefault));

  async function submit(e?: Event) {
    e?.preventDefault();
    if (!server || !username || !password) {
      error = "Enter your server address, username and password.";
      return;
    }
    busy = true;
    error = "";
    try {
      app.config = await api.login(server, username, password);
      app.signedIn = true;
      app.stack = [{ name: "home" }];
      await loadHome();
    } catch (err) {
      error = String(err);
    } finally {
      busy = false;
    }
  }
</script>

<div class="login">
  <form onsubmit={submit} data-nav-group>
    <h1>Tunes</h1>
    <p class="dim">Sign in to your Navidrome server</p>
    <label>
      <span>Server</span>
      <input data-nav data-nav-default bind:value={server} placeholder="https://music.example.com" autocomplete="url" spellcheck="false" />
    </label>
    <label>
      <span>Username</span>
      <input data-nav bind:value={username} autocomplete="username" spellcheck="false" />
    </label>
    <label>
      <span>Password</span>
      <input data-nav type="password" bind:value={password} autocomplete="current-password" />
    </label>
    {#if error}<p class="error">{error}</p>{/if}
    <button class="btn primary" data-nav type="submit" disabled={busy}>{busy ? "Signing in…" : "Sign in"}</button>
    <p class="faint tip">Tip: on a handheld, press Steam + X for the on-screen keyboard. Your password is kept in the system credential store so you stay signed in.</p>
  </form>
</div>

<style>
  .login { height: 100%; display: grid; place-items: center; }
  form {
    width: 520px; display: flex; flex-direction: column; gap: 18px; padding: 44px 48px; border-radius: var(--radius);
    background: var(--surface); box-shadow: 0 30px 80px rgba(0,0,0,.55); animation: page-in var(--t-fade) var(--ease-out);
  }
  h1 { margin: 0; font-size: 34px; font-weight: 700; }
  p { margin: 0; }
  label { display: flex; flex-direction: column; gap: 6px; }
  label span { font-size: 11px; font-weight: 700; letter-spacing: .08em; text-transform: uppercase; color: var(--text-dim); }
  input {
    padding: 13px 14px; border-radius: var(--radius-sm); border: 1px solid transparent; background: #32353c;
    color: var(--text); font-size: 16px; transition: border-color var(--t-focus), background var(--t-focus);
  }
  input:focus { border-color: var(--accent); background: #3a3e46; }
  .btn { margin-top: 8px; min-height: 48px; font-size: 16px; }
  .error { color: #ff7a70; font-size: 14px; }
  .tip { font-size: 13px; line-height: 1.5; }
</style>
