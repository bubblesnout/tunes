// Maps raw inputs (gamepad buttons from the backend, keyboard keys) to app actions.
// Bindings are user-remappable and stored in config as { action: ["Pad:A", "Key:Enter"] }.

import { on } from "./api";

export type Action =
  | "up" | "down" | "left" | "right"
  | "select" | "back" | "options"
  | "playPause" | "prevTrack" | "nextTrack"
  | "volDown" | "volUp"
  | "nowPlaying" | "menu"
  | "pageUp" | "pageDown";

export const ACTIONS: { id: Action; label: string }[] = [
  { id: "up", label: "Navigate up" },
  { id: "down", label: "Navigate down" },
  { id: "left", label: "Navigate left" },
  { id: "right", label: "Navigate right" },
  { id: "select", label: "Select" },
  { id: "back", label: "Back" },
  { id: "options", label: "Options (queue / play next)" },
  { id: "playPause", label: "Play / pause" },
  { id: "prevTrack", label: "Previous track" },
  { id: "nextTrack", label: "Next track" },
  { id: "volDown", label: "Volume down" },
  { id: "volUp", label: "Volume up" },
  { id: "menu", label: "Main menu" },
  { id: "nowPlaying", label: "Now Playing" },
  { id: "pageUp", label: "Scroll up a page" },
  { id: "pageDown", label: "Scroll down a page" },
];

export type Bindings = Record<Action, string[]>;

export const DEFAULT_BINDINGS: Bindings = {
  up: ["Pad:DPadUp", "Pad:LStickUp", "Key:ArrowUp"],
  down: ["Pad:DPadDown", "Pad:LStickDown", "Key:ArrowDown"],
  left: ["Pad:DPadLeft", "Pad:LStickLeft", "Key:ArrowLeft"],
  right: ["Pad:DPadRight", "Pad:LStickRight", "Key:ArrowRight"],
  select: ["Pad:A", "Key:Enter"],
  back: ["Pad:B", "Key:Escape", "Key:Backspace"],
  options: ["Pad:Menu", "Key:KeyO"],
  playPause: ["Pad:Y", "Key:Space"],
  prevTrack: ["Pad:LB", "Key:BracketLeft"],
  nextTrack: ["Pad:RB", "Key:BracketRight"],
  volDown: ["Pad:LT", "Key:Minus"],
  volUp: ["Pad:RT", "Key:Equal"],
  nowPlaying: ["Pad:X", "Key:KeyN"],
  menu: ["Pad:View", "Key:KeyM"],
  pageUp: ["Pad:RStickUp", "Key:PageUp"],
  pageDown: ["Pad:RStickDown", "Key:PageDown"],
};

/** Actions that should fire repeatedly while held. */
const REPEATABLE = new Set<Action>(["up", "down", "left", "right", "volDown", "volUp", "pageUp", "pageDown"]);

let bindings: Bindings = structuredClone(DEFAULT_BINDINGS);
let lookup = new Map<string, Action[]>();

function rebuild() {
  lookup = new Map();
  for (const [action, inputs] of Object.entries(bindings) as [Action, string[]][]) {
    for (const i of inputs) {
      const list = lookup.get(i) ?? [];
      list.push(action);
      lookup.set(i, list);
    }
  }
}
rebuild();

/** Bump when default layout changes incompatibly; older saved bindings are discarded. */
export const BINDINGS_VERSION = 2;

export function setBindings(b: (Partial<Bindings> & { _v?: number }) | null | undefined) {
  const saved = b && b._v === BINDINGS_VERSION ? b : {};
  const merged = structuredClone(DEFAULT_BINDINGS);
  for (const a of Object.keys(merged) as Action[]) if (Array.isArray(saved[a])) merged[a] = saved[a]!;
  bindings = merged;
  rebuild();
}

export function getBindings(): Bindings {
  return structuredClone(bindings);
}

export type Device = "pad" | "key";
type Handler = (action: Action, repeat: boolean) => void;
type CaptureHandler = (input: string) => void;

let handler: Handler = () => {};
let capture: CaptureHandler | null = null;
let deviceListeners: ((d: Device) => void)[] = [];
let lastDevice: Device = "pad";

export function onAction(h: Handler) {
  handler = h;
}

/** Next raw input (pad button or key) goes to `cb` instead of triggering an action. */
export function captureNext(cb: CaptureHandler | null) {
  capture = cb;
}

export function onDeviceChange(cb: (d: Device) => void) {
  deviceListeners.push(cb);
  cb(lastDevice);
}

function setDevice(d: Device) {
  if (d !== lastDevice) {
    lastDevice = d;
    deviceListeners.forEach((l) => l(d));
  }
}

function dispatch(input: string, repeat: boolean): boolean {
  if (capture && !repeat) {
    const cb = capture;
    capture = null;
    cb(input);
    return true;
  }
  const actions = lookup.get(input);
  if (!actions) return false;
  for (const a of actions) {
    if (repeat && !REPEATABLE.has(a)) continue;
    handler(a, repeat);
  }
  return true;
}

function isTextField(el: EventTarget | null): boolean {
  return el instanceof HTMLInputElement && !["checkbox", "radio", "button", "range"].includes(el.type);
}

export async function initInput() {
  await on<{ button: string; pressed: boolean; repeat: boolean }>("pad", (e) => {
    if (!e.pressed) return;
    setDevice("pad");
    dispatch(`Pad:${e.button}`, e.repeat);
  });

  window.addEventListener(
    "keydown",
    (e) => {
      setDevice("key");
      if (e.key === "F11") {
        e.preventDefault();
        window.dispatchEvent(new CustomEvent("toggle-fullscreen"));
        return;
      }
      // While typing, only let navigation-type keys through.
      if (isTextField(e.target) && !capture) {
        if (!["Escape", "Enter", "ArrowUp", "ArrowDown", "Tab"].includes(e.key)) return;
        if (e.key === "Tab") return;
      }
      if (dispatch(`Key:${e.code}`, e.repeat)) e.preventDefault();
    },
    { capture: true },
  );
  window.addEventListener("mousedown", () => setDevice("key"));
}

/** Human-friendly label for an input id, e.g. "Pad:LB" -> "LB", "Key:BracketLeft" -> "[". */
export function inputLabel(input: string): string {
  const [kind, name] = input.split(":");
  if (kind === "Pad") {
    return (
      {
        DPadUp: "D-pad ↑", DPadDown: "D-pad ↓", DPadLeft: "D-pad ←", DPadRight: "D-pad →",
        LStickUp: "L-stick ↑", LStickDown: "L-stick ↓", LStickLeft: "L-stick ←", LStickRight: "L-stick →",
        RStickUp: "R-stick ↑", RStickDown: "R-stick ↓", RStickLeft: "R-stick ←", RStickRight: "R-stick →",
        Menu: "☰", View: "⧉", LS: "L3", RS: "R3",
      } as Record<string, string>
    )[name] ?? name;
  }
  const keys: Record<string, string> = {
    BracketLeft: "[", BracketRight: "]", Minus: "-", Equal: "=", Space: "Space", Escape: "Esc",
    ArrowUp: "↑", ArrowDown: "↓", ArrowLeft: "←", ArrowRight: "→", Backspace: "⌫", Enter: "Enter",
  };
  return keys[name] ?? name.replace(/^Key/, "").replace(/^Digit/, "");
}

/** First binding for `action` on the given device, for on-screen button hints. */
export function hintFor(action: Action, device: Device): string | null {
  const prefix = device === "pad" ? "Pad:" : "Key:";
  return bindings[action]?.find((i) => i.startsWith(prefix)) ?? null;
}
