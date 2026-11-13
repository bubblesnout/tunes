// Generates the Steam library artwork for the "Add Tunes to Steam" shortcut.
//
//   node scripts/steam-art.mjs            → art/steam/*.svg + *.png
//
// Style: dark navy base, heavily blurred colour fields (Steam blue / cyan / violet),
// faint blurred music motifs (vinyl grooves, equaliser bars, notes) and the Tunes mark.
// Text is rendered with Motiva Sans from public/fonts (falls back to a system sans).

import { Resvg } from "@resvg/resvg-js";
import { existsSync, mkdirSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const outDir = join(root, "art", "steam");
mkdirSync(outDir, { recursive: true });

const C = {
  base1: "#0f1a2b",
  base2: "#080c14",
  blue: "#1a9fff",
  cyan: "#00ccff",
  violet: "#6a4cff",
  navy: "#1b2a6b",
  white: "#ffffff",
};

// Deterministic pseudo-random so renders are stable.
function rng(seed) {
  return () => ((seed = (seed * 16807) % 2147483647) / 2147483647);
}

// ---------- building blocks --------------------------------------------------------------

function defs(w, h) {
  const m = Math.min(w, h);
  // Filter regions pinned to the canvas: resvg panics when an object-bbox region
  // extends far outside the image.
  const region = `filterUnits="userSpaceOnUse" x="0" y="0" width="${w}" height="${h}"`;
  return `
  <defs>
    <linearGradient id="base" x1="0" y1="0" x2="1" y2="1">
      <stop offset="0" stop-color="${C.base1}"/>
      <stop offset="1" stop-color="${C.base2}"/>
    </linearGradient>
    <linearGradient id="tile" x1="0" y1="0" x2="1" y2="1">
      <stop offset="0" stop-color="${C.blue}"/>
      <stop offset="1" stop-color="${C.navy}"/>
    </linearGradient>
    <radialGradient id="tileHi" cx="200" cy="140" r="760" gradientUnits="userSpaceOnUse">
      <stop offset="0" stop-color="#fff" stop-opacity=".22"/>
      <stop offset="1" stop-color="#fff" stop-opacity="0"/>
    </radialGradient>
    <linearGradient id="word" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0" stop-color="#ffffff"/>
      <stop offset="1" stop-color="#cfe8ff"/>
    </linearGradient>
    <linearGradient id="bars" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0" stop-color="${C.cyan}" stop-opacity=".55"/>
      <stop offset="1" stop-color="${C.blue}" stop-opacity="0"/>
    </linearGradient>
    <radialGradient id="vignette" cx=".5" cy=".45" r=".75">
      <stop offset=".55" stop-color="#000" stop-opacity="0"/>
      <stop offset="1" stop-color="#000" stop-opacity=".55"/>
    </radialGradient>
        <filter id="softBlur" ${region}><feGaussianBlur stdDeviation="${m * 0.006}"/></filter>
    <filter id="midBlur" ${region}><feGaussianBlur stdDeviation="${m * 0.007}"/></filter>
    <filter id="shadow" ${region}>
      <feDropShadow dx="0" dy="${m * 0.02}" stdDeviation="${m * 0.03}" flood-color="#000" flood-opacity=".55"/>
    </filter>
  </defs>`;
}

/** Background: gradient base + soft colour fields + vignette. The fields are radial
 *  gradients rather than blurred shapes: perfectly smooth at any size (resvg's large-radius
 *  blur produces blocky artefacts). */
let blobId = 0;
function background(w, h, blobs) {
  let grads = "";
  let shapes = "";
  for (const [cx, cy, r, color, op] of blobs) {
    const id = `blob${blobId++}`;
    const rr = r * Math.max(w, h) * 1.6;
    grads += `<radialGradient id="${id}" cx="${cx * w}" cy="${cy * h}" r="${rr}" gradientUnits="userSpaceOnUse">
      <stop offset="0" stop-color="${color}" stop-opacity="${op}"/>
      <stop offset=".45" stop-color="${color}" stop-opacity="${op * 0.45}"/>
      <stop offset="1" stop-color="${color}" stop-opacity="0"/>
    </radialGradient>`;
    shapes += `<rect width="${w}" height="${h}" fill="url(#${id})"/>`;
  }
  return `
  <defs>${grads}</defs>
  <rect width="${w}" height="${h}" fill="url(#base)"/>
  ${shapes}`;
}

/** Concentric vinyl grooves, softly blurred. */
function vinyl(cx, cy, r, opacity = 0.09) {
  let rings = "";
  for (let i = 0; i < 22; i++) {
    const rr = r * (0.28 + (i / 22) * 0.72);
    rings += `<circle cx="${cx}" cy="${cy}" r="${rr}" fill="none" stroke="#fff" stroke-width="${r * 0.006}" opacity="${0.35 + (i % 3) * 0.2}"/>`;
  }
  return `
  <g opacity="${opacity}" filter="url(#softBlur)">
    <circle cx="${cx}" cy="${cy}" r="${r}" fill="#000" opacity=".25"/>
    ${rings}
    <circle cx="${cx}" cy="${cy}" r="${r * 0.2}" fill="${C.blue}" opacity=".9"/>
    <circle cx="${cx}" cy="${cy}" r="${r * 0.025}" fill="#000"/>
  </g>`;
}

/** Equaliser bars rising from a baseline, blurred so they read as texture. */
function equaliser(x, baseline, width, maxH, count, seed, opacity = 0.5) {
  const r = rng(seed);
  const gap = width / count;
  const bw = gap * 0.58;
  let bars = "";
  for (let i = 0; i < count; i++) {
    const t = i / count;
    const env = 0.35 + 0.65 * Math.sin(Math.PI * t) ** 0.7;
    const hgt0 = maxH * env * (0.25 + 0.75 * (0.5 + 0.5 * Math.sin(i * 0.7 + seed)) * (0.6 + 0.4 * r()));
    const hgt = Math.max(bw * 1.5, hgt0);
    bars += `<rect x="${x + i * gap}" y="${baseline - hgt}" width="${bw}" height="${hgt}" rx="${bw / 2}" fill="url(#bars)"/>`;
  }
  return `<g opacity="${opacity}" filter="url(#midBlur)">${bars}</g>`;
}

/** A single eighth-note glyph (for scattered decoration). */
function note(x, y, s, rot, opacity) {
  return `<g transform="translate(${x} ${y}) rotate(${rot}) scale(${s})" opacity="${opacity}">
    <path d="M10 -60 V20" stroke="#fff" stroke-width="7" stroke-linecap="round"/>
    <path d="M10 -60 Q34 -50 36 -26" stroke="#fff" stroke-width="7" fill="none" stroke-linecap="round"/>
    <ellipse cx="-4" cy="22" rx="17" ry="13" fill="#fff" transform="rotate(-20 -4 22)"/>
  </g>`;
}

function notes(w, h, seed, n, minS, maxS, opacity) {
  const r = rng(seed);
  let g = "";
  for (let i = 0; i < n; i++) {
    g += note(r() * w, r() * h, minS + r() * (maxS - minS), -25 + r() * 50, opacity * (0.5 + r() * 0.5));
  }
  return `<g filter="url(#softBlur)">${g}</g>`;
}

/** The Tunes mark (app icon): gradient rounded tile with beamed notes. `size` = tile edge. */
function mark(x, y, size, withShadow = true) {
  const s = size / 1024;
  return `
  <g ${withShadow ? 'filter="url(#shadow)"' : ""}><g transform="translate(${x} ${y}) scale(${s})">
    <rect x="32" y="32" width="960" height="960" rx="220" fill="url(#tile)"/>
    <rect x="32" y="32" width="960" height="960" rx="220" fill="url(#tileHi)"/>
    <!-- Notes nudged left/down so they sit optically centred: their outline centre was
         ~31 units high and the beam/stems pull their visual weight ~40 units right. -->
    <g transform="translate(-20 20)">
      <path d="M420 250 L760 190 L760 640" fill="none" stroke="#fff" stroke-width="64" stroke-linejoin="round" stroke-linecap="round"/>
      <path d="M420 250 L420 700" fill="none" stroke="#fff" stroke-width="64" stroke-linecap="round"/>
      <ellipse cx="330" cy="720" rx="110" ry="84" fill="#fff"/>
      <ellipse cx="670" cy="660" rx="110" ry="84" fill="#fff"/>
    </g>
  </g></g>`;
}

function wordmark(x, y, size, anchor = "start") {
  return `<text x="${x}" y="${y}" font-family="Motiva Sans" font-weight="700" font-size="${size}"
    letter-spacing="${size * -0.01}" fill="url(#word)" text-anchor="${anchor}">Tunes</text>`;
}

function tagline(x, y, size, anchor = "start") {
  return `<text x="${x}" y="${y}" font-family="Motiva Sans" font-weight="500" font-size="${size}"
    letter-spacing="${size * 0.22}" fill="#9fc9f0" opacity=".85" text-anchor="${anchor}">FOR NAVIDROME</text>`;
}

const svg = (w, h, body) =>
  `<svg xmlns="http://www.w3.org/2000/svg" width="${w}" height="${h}" viewBox="0 0 ${w} ${h}">${defs(w, h)}${body}</svg>`;

// ---------- assets -----------------------------------------------------------------------

const assets = {
  // Library grid (portrait capsule)
  capsule: () => {
    const w = 600, h = 900;
    return svg(w, h, `
      ${background(w, h, [
        [0.15, 0.1, 0.45, C.blue, 0.55],
        [0.95, 0.45, 0.4, C.violet, 0.45],
        [0.3, 1.0, 0.45, C.cyan, 0.3],
      ])}
      ${vinyl(w * 0.88, h * 0.95, w * 0.7, 0.1)}
      ${notes(w, h * 0.6, 7, 9, 0.35, 0.7, 0.07)}
      ${equaliser(-10, h + 20, w + 20, h * 0.28, 26, 3, 0.45)}
      <rect width="${w}" height="${h}" fill="url(#vignette)"/>
      ${mark(w / 2 - 130, h * 0.2, 260)}
      ${wordmark(w / 2, h * 0.64, 132, "middle")}
      ${tagline(w / 2 + 6, h * 0.64 + 52, 22, "middle")}
    `);
  },

  // Recent games row / Big Picture (wide capsule)
  wide: () => {
    const w = 920, h = 430;
    return svg(w, h, `
      ${background(w, h, [
        [0.08, 0.2, 0.32, C.blue, 0.6],
        [0.7, 0.0, 0.3, C.violet, 0.4],
        [0.95, 0.95, 0.35, C.cyan, 0.32],
      ])}
      ${vinyl(w * 0.86, h * 0.5, h * 0.75, 0.1)}
      ${notes(w, h, 11, 10, 0.3, 0.55, 0.07)}
      ${equaliser(w * 0.02, h + 10, w * 0.96, h * 0.38, 40, 5, 0.4)}
      <rect width="${w}" height="${h}" fill="url(#vignette)"/>
      ${mark(w * 0.12, h / 2 - 105, 210)}
      ${wordmark(w * 0.12 + 245, h / 2 + 34, 128)}
      ${tagline(w * 0.12 + 252, h / 2 + 84, 20)}
    `);
  },

  // App page banner (no text — Steam overlays the logo)
  hero: () => {
    const w = 3840, h = 1240;
    return svg(w, h, `
      ${background(w, h, [
        [0.1, 0.1, 0.22, C.blue, 0.55],
        [0.45, 0.9, 0.2, C.navy, 0.6],
        [0.7, 0.15, 0.2, C.violet, 0.4],
        [0.95, 0.75, 0.22, C.cyan, 0.3],
      ])}
      ${vinyl(w * 0.82, h * 0.55, h * 0.95, 0.09)}
      ${notes(w, h, 23, 22, 1.2, 2.4, 0.06)}
      ${equaliser(-20, h + 30, w + 40, h * 0.45, 96, 9, 0.42)}
      <rect width="${w}" height="${h}" fill="url(#vignette)"/>
    `);
  },

  // Transparent logo, overlaid on the hero by Steam
  logo: () => {
    const w = 1280, h = 360;
    return svg(w, h, `
      ${mark(185, 30, 300)}
      ${wordmark(535, 238, 200)}
    `).replace(/<svg ([^>]*)>/, `<svg $1><!-- transparent -->`);
  },

  // Small list icon
  icon: () => {
    const w = 256, h = 256;
    return svg(w, h, mark(0, 0, 256, false));
  },
};

// ---------- render -----------------------------------------------------------------------

const fontDir = join(root, "public", "fonts");
const fontFiles = ["MotivaSans-Bold.ttf", "MotivaSans-Medium.ttf", "MotivaSans-Regular.ttf"]
  .map((f) => join(fontDir, f))
  .filter(existsSync);
if (!fontFiles.length) console.warn("Motiva Sans not found in public/fonts — text will use a system font.");

for (const [name, build] of Object.entries(assets)) {
  const source = build();
  writeFileSync(join(outDir, `${name}.svg`), source);
  const png = new Resvg(source, {
    font: { fontFiles, loadSystemFonts: fontFiles.length === 0, defaultFontFamily: "Motiva Sans" },
    background: name === "logo" || name === "icon" ? "rgba(0,0,0,0)" : undefined,
  })
    .render()
    .asPng();
  writeFileSync(join(outDir, `${name}.png`), png);
  console.log(`${name}.png  ${(png.length / 1024).toFixed(0)} KB`);
}
