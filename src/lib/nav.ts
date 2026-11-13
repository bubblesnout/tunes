// Spatial (directional) focus navigation for gamepad/keyboard.
//
// Any element with `data-nav` is focusable. Elements inside the top-most `[data-nav-scope]`
// (e.g. a modal menu) take priority. `[data-nav-group]` containers remember their last
// focused child, so moving between shelves returns you to where you were.

export type Dir = "up" | "down" | "left" | "right";

const lastInGroup = new WeakMap<Element, HTMLElement>();

function scopeRoot(): ParentNode {
  const scopes = document.querySelectorAll<HTMLElement>("[data-nav-scope]");
  return scopes.length ? scopes[scopes.length - 1] : document;
}

function focusables(root: ParentNode): HTMLElement[] {
  return Array.from(root.querySelectorAll<HTMLElement>("[data-nav]")).filter(
    (el) => !(el as HTMLButtonElement).disabled && el.offsetParent !== null && !el.closest("[inert]"),
  );
}

function groupOf(el: Element): Element | null {
  return el.closest("[data-nav-group]");
}

export function current(): HTMLElement | null {
  const a = document.activeElement as HTMLElement | null;
  if (!a || !a.matches("[data-nav]")) return null;
  const root = scopeRoot();
  if (root !== document && !(root as HTMLElement).contains(a)) return null;
  return a;
}

export function focusEl(el: HTMLElement | null | undefined, scroll = true, behavior: ScrollBehavior = "smooth") {
  if (!el) return;
  el.focus({ preventScroll: true });
  if (scroll) el.scrollIntoView({ block: "nearest", inline: "nearest", behavior });
}

/** Focus the scope's preferred element (`data-nav-default`), or its first focusable. */
export function focusDefault() {
  const root = scopeRoot();
  const preferred = root.querySelector<HTMLElement>("[data-nav-default]");
  const list = focusables(root);
  focusEl(preferred && list.includes(preferred) ? preferred : list[0]);
}

export function ensureFocus() {
  if (!current()) focusDefault();
}

export function move(dir: Dir, behavior: ScrollBehavior = "smooth"): boolean {
  const cur = current();
  if (!cur) {
    focusDefault();
    return true;
  }
  const r = cur.getBoundingClientRect();
  const cx = r.left + r.width / 2;
  const cy = r.top + r.height / 2;
  let best: HTMLElement | null = null;
  let bestScore = Infinity;
  for (const el of focusables(scopeRoot())) {
    if (el === cur) continue;
    const c = el.getBoundingClientRect();
    const ex = c.left + c.width / 2;
    const ey = c.top + c.height / 2;
    let primary: number;
    let perp: number;
    // Candidates must lie genuinely in the direction of travel. A small overlap is tolerated
    // because the focused tile is scaled up (its rect grows into its neighbours).
    const tx = r.width * 0.15;
    const ty = r.height * 0.15;
    switch (dir) {
      case "right":
        if (c.left < r.right - tx || ex <= cx) continue;
        primary = Math.max(0, c.left - r.right);
        perp = Math.abs(ey - cy);
        break;
      case "left":
        if (c.right > r.left + tx || ex >= cx) continue;
        primary = Math.max(0, r.left - c.right);
        perp = Math.abs(ey - cy);
        break;
      case "down":
        if (c.top < r.bottom - ty || ey <= cy) continue;
        primary = Math.max(0, c.top - r.bottom);
        perp = Math.abs(ex - cx);
        break;
      case "up":
        if (c.bottom > r.top + ty || ey >= cy) continue;
        primary = Math.max(0, r.top - c.bottom);
        perp = Math.abs(ex - cx);
        break;
    }
    // Moving sideways strongly prefers the same row; moving vertically prefers the nearest row.
    const score = dir === "left" || dir === "right" ? primary + perp * 4 : primary * 2 + perp;
    if (score < bestScore) {
      bestScore = score;
      best = el;
    }
  }
  if (!best) return false;
  const g = groupOf(best);
  if (g && g !== groupOf(cur)) {
    // Entering a group from outside: prefer its designated entry point (e.g. an album's
    // Play button), otherwise return to wherever focus last was in that group.
    const entry = g.querySelector<HTMLElement>("[data-nav-entry]");
    const remembered = lastInGroup.get(g);
    if (entry && focusables(g).includes(entry)) best = entry;
    else if (remembered && remembered.isConnected && remembered.offsetParent !== null) best = remembered;
  }
  focusEl(best, true, behavior);
  return true;
}

/** Scroll the main scroll container by a page. */
export function page(delta: number) {
  const el = document.querySelector<HTMLElement>("[data-scroll-main]");
  if (!el) return;
  el.scrollBy({ top: delta * el.clientHeight * 0.8, behavior: "smooth" });
}

// Track group memory + pointer-driven focus.
document.addEventListener("focusin", (e) => {
  const t = e.target as HTMLElement;
  if (!t.matches?.("[data-nav]")) return;
  const g = groupOf(t);
  if (g) lastInGroup.set(g, t);
});

document.addEventListener("pointermove", (e) => {
  const t = (e.target as HTMLElement).closest?.<HTMLElement>("[data-nav]");
  if (t && document.activeElement !== t) focusEl(t, false);
});
