<script lang="ts">
  // Album or playlist page, laid out like a Big Picture game page:
  // hero banner → action bar (Play + stats) → content panel.
  import { api, fmtTime, type Song } from "../api";
  import { app, toast } from "../app.svelte";
  import { playSongs } from "../actions";
  import { focusDefault } from "../nav";
  import ActionBar from "../components/ActionBar.svelte";
  import Hero from "../components/Hero.svelte";
  import TrackList from "../components/TrackList.svelte";
  import { tick } from "svelte";

  let { kind, id }: { kind: "album" | "playlist"; id: string } = $props();

  let title = $state("");
  let artist = $state("");
  let cover = $state<string | undefined>();
  let songs = $state<Song[]>([]);
  let stats = $state<{ label: string; value: string }[]>([]);
  let loading = $state(true);

  $effect(() => {
    load(kind, id);
  });

  async function load(k: typeof kind, i: string) {
    loading = true;
    try {
      if (k === "album") {
        const a = await api.album(i);
        title = a.name;
        artist = a.artist ?? "";
        cover = a.coverArt;
        songs = a.song ?? [];
        stats = [
          { label: "Released", value: a.year ? String(a.year) : "—" },
          { label: "Tracks", value: String(songs.length) },
          { label: "Length", value: fmtTime(a.duration ?? songs.reduce((t, s) => t + (s.duration ?? 0), 0)) },
          ...(a.genre ? [{ label: "Genre", value: a.genre }] : []),
        ];
      } else {
        const p = await api.playlist(i);
        title = p.name;
        artist = p.owner ? `Playlist by ${p.owner}` : "Playlist";
        cover = p.coverArt;
        songs = p.entry ?? [];
        stats = [
          { label: "Tracks", value: String(songs.length) },
          { label: "Length", value: fmtTime(songs.reduce((t, s) => t + (s.duration ?? 0), 0)) },
        ];
      }
      app.backdrop = null;
    } catch (e) {
      toast(String(e));
    } finally {
      loading = false;
      await tick();
      focusDefault();
    }
  }
</script>

<div class="collection page">
  <Hero {cover} kind={kind === "album" ? "Album" : "Playlist"} {title} subtitle={artist} />
  <ActionBar {title} {stats} disabled={loading} songs={() => songs} />
  <TrackList {songs} numbering={kind === "album" ? "track" : "index"} />
</div>

<style>
  .collection { padding-bottom: 40px; }
</style>
