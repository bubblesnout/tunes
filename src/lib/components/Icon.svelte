<script lang="ts">
  let { name, size = 22 }: { name: string; size?: number } = $props();
  const paths: Record<string, string> = {
    play: "M8 5v14l11-7z",
    pause: "M6 5h4v14H6zM14 5h4v14h-4z",
    next: "M6 18l8.5-6L6 6v12zM16 6h2v12h-2z",
    prev: "M6 6h2v12H6zm3.5 6 8.5 6V6z",
    shuffle: "M10.6 9.2 5.4 4 4 5.4l5.2 5.2 1.4-1.4zM14.5 4l2 2L4 18.6 5.4 20 18 7.5l2 2V4h-5.5zm.3 9.4-1.4 1.4 3.1 3.1-2 2.1H20v-5.5l-2 2-3.2-3.1z",
    repeat: "M7 7h10v3l4-4-4-4v3H5v6h2V7zm10 10H7v-3l-4 4 4 4v-3h12v-6h-2v4z",
    repeatOne: "M7 7h10v3l4-4-4-4v3H5v6h2V7zm10 10H7v-3l-4 4 4 4v-3h12v-6h-2v4zm-4-2V9h-1l-2 1v1h1.5v4H13z",
    volume: "M3 9v6h4l5 5V4L7 9H3zm13.5 3A4.5 4.5 0 0 0 14 8v8a4.5 4.5 0 0 0 2.5-4zM14 3.2v2.1a7 7 0 0 1 0 13.4v2.1a9 9 0 0 0 0-17.6z",
    mute: "M3 9v6h4l5 5V4L7 9H3z",
    home: "M10 20v-6h4v6h5v-8h3L12 3 2 12h3v8z",
    music: "M12 3v10.6A4 4 0 1 0 14 17V7h4V3h-6z",
    settings: "M19.4 13a7.5 7.5 0 0 0 0-2l2.1-1.6-2-3.5-2.5 1a7.6 7.6 0 0 0-1.7-1L15 3h-4l-.4 2.9a7.6 7.6 0 0 0-1.7 1l-2.5-1-2 3.5L6.6 11a7.5 7.5 0 0 0 0 2l-2.1 1.6 2 3.5 2.5-1c.5.4 1.1.7 1.7 1L11 21h4l.4-2.9c.6-.3 1.2-.6 1.7-1l2.5 1 2-3.5-2.2-1.6zM13 15.5a3.5 3.5 0 1 1 0-7 3.5 3.5 0 0 1 0 7z",
    queue: "M3 6h12v2H3zm0 4h12v2H3zm0 4h8v2H3zm14-4v8l5-4z",
    plus: "M19 13h-6v6h-2v-6H5v-2h6V5h2v6h6z",
    album: "M12 2a10 10 0 1 0 0 20 10 10 0 0 0 0-20zm0 14.5a4.5 4.5 0 1 1 0-9 4.5 4.5 0 0 1 0 9zm0-5.5a1 1 0 1 0 0 2 1 1 0 0 0 0-2z",
    refresh: "M17.7 6.3A8 8 0 1 0 19.7 14h-2.1A6 6 0 1 1 12 6c1.7 0 3.1.7 4.2 1.8L13 11h7V4l-2.3 2.3z",
    gamepad: "M7.5 6h9A5.5 5.5 0 0 1 22 11.5V14a4 4 0 0 1-7.3 2.3L13.5 15h-3l-1.2 1.3A4 4 0 0 1 2 14v-2.5A5.5 5.5 0 0 1 7.5 6zM7 9v1.5H5.5v2H7V14h2v-1.5h1.5v-2H9V9H7zm8.5 0.5a1 1 0 1 0 0 2 1 1 0 0 0 0-2zm2 2a1 1 0 1 0 0 2 1 1 0 0 0 0-2z",
    keyboard: "M3 6h18a1 1 0 0 1 1 1v10a1 1 0 0 1-1 1H3a1 1 0 0 1-1-1V7a1 1 0 0 1 1-1zm2 3v2h2V9H5zm3 0v2h2V9H8zm3 0v2h2V9h-2zm3 0v2h2V9h-2zm3 0v2h2V9h-2zM7 13v2h10v-2H7z",
    user: "M12 12a4.5 4.5 0 1 0 0-9 4.5 4.5 0 0 0 0 9zm0 2c-4 0-8 2-8 5v2h16v-2c0-3-4-5-8-5z",
    library: "M4 4h7v7H4zm9 0h7v7h-7zM4 13h7v7H4zm9 0h7v7h-7z",
    power: "M13 3h-2v10h2V3zm4.8 2.2-1.4 1.4A7 7 0 1 1 7.6 6.6L6.2 5.2A9 9 0 1 0 17.8 5.2z",
  };
</script>

<svg viewBox="0 0 24 24" width={size} height={size} fill="currentColor" aria-hidden="true">
  <path d={paths[name] ?? ""} />
</svg>
