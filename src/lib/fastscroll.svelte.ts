// Steam-style fast scrolling through A–Z lists.
//
// Holding up/down on an item that has a `data-letter` (the library grids) dims the screen and
// shows the current letter large in the middle, as Big Picture does in the All Games list.
// Hold for longer and each repeat jumps a whole letter instead of a row.

import { current, focusEl, move, type Dir } from "./nav";

/** Repeats (85 ms apart on a gamepad) before moving a letter at a time instead of a row. */
const LETTER_JUMP_AFTER = 8;
const HIDE_AFTER_MS = 450;

export const fastScroll = $state({ letter: "", visible: false });

let held = 0;
let hideTimer: ReturnType<typeof setTimeout> | undefined;

function hide() {
  clearTimeout(hideTimer);
  held = 0;
  fastScroll.visible = false;
}

/** First item of the letter group after (down) or before (up) the current one. */
function jumpLetter(cur: HTMLElement, dir: "up" | "down"): boolean {
  const group = cur.closest("[data-nav-group]") ?? document;
  const items = Array.from(group.querySelectorAll<HTMLElement>("[data-letter]"));
  const i = items.indexOf(cur);
  if (i < 0) return false;
  const letter = cur.dataset.letter;
  let target: HTMLElement | undefined;
  if (dir === "down") {
    target = items.slice(i + 1).find((el) => el.dataset.letter !== letter);
  } else {
    // Start of this letter first, then the start of the one before.
    let start = i;
    while (start > 0 && items[start - 1].dataset.letter === letter) start--;
    if (start < i) target = items[start];
    else if (start > 0) {
      const prev = items[start - 1].dataset.letter;
      let s = start - 1;
      while (s > 0 && items[s - 1].dataset.letter === prev) s--;
      target = items[s];
    }
  }
  if (!target) return false;
  focusEl(target, true, "instant");
  return true;
}

/**
 * Handle up/down. Returns false when it doesn't apply (not a held press on a lettered item),
 * so the caller falls back to normal navigation.
 */
export function verticalMove(dir: Dir, repeat: boolean): boolean {
  if (dir !== "up" && dir !== "down") return false;
  const cur = current();
  if (!repeat || !cur?.dataset.letter) {
    if (fastScroll.visible || held) hide();
    return false;
  }
  held++;
  // At either end of the list there's nothing further to move to; the press is still ours.
  if (held > LETTER_JUMP_AFTER) jumpLetter(cur, dir);
  else move(dir, "instant");
  const now = current();
  if (now?.dataset.letter) {
    fastScroll.letter = now.dataset.letter;
    fastScroll.visible = true;
  }
  clearTimeout(hideTimer);
  hideTimer = setTimeout(hide, HIDE_AFTER_MS);
  return true;
}
