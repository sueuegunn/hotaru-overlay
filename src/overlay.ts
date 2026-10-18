// The overlay window. It draws the board and nothing else - no input, no state
// of its own. "Off" is drawn rather than hidden: the window stays up (showing
// and hiding it steals focus on Windows) and simply renders nothing.
//
// Two speeds: `build()` makes the keycaps and runs only when what they say
// changes (layer, keymap, window size); `paint()` only flips `.pressed` and
// runs on every report from the keyboard, up to 250 a second.
import { listen } from "@tauri-apps/api/event";

import { getFiles, getState, type Files, type Place, type StatePayload } from "./api";
import { isTransparent, legend } from "./keycodes";

const panel = document.getElementById("panel") as HTMLDivElement;
const layerLabel = document.getElementById("layer") as HTMLSpanElement;
const linkLabel = document.getElementById("link") as HTMLSpanElement;
const board = document.getElementById("board") as HTMLDivElement;

/**
 * The shortest time a key stays lit. A key goes dark the moment it is released,
 * unless it has not yet been lit this long - that is only for taps quicker than
 * a report, which the firmware latches into a single one. A tail after every
 * release (120ms, the first version) read as lag.
 */
const MIN_LIT_MS = 60;

/** `k<cell>` for a key (cell = row * cols + col), `e<index>:<dir>` for an encoder half. */
type Id = string;

let files: Files;
let current: StatePayload;

/** Keycap elements by id, and the encoders' halves. */
const caps = new Map<Id, HTMLElement>();
/** When each lit key was first seen in this press. */
const since = new Map<Id, number>();
/** What the last build() drew, so paint() knows when a rebuild is due. */
let built = "";
let expiry: number | undefined;

/**
 * Legend for a cell: the topmost active layer whose cell is not KC_TRNS, as
 * QMK itself resolves it. `dim` when it came from a layer below the top.
 */
function resolve(pick: (layer: number) => string | undefined): { text: string; dim: boolean } {
  for (let l = current.layer; l >= 0; l--) {
    if (l !== current.layer && !(current.active & (1 << l))) continue;
    const code = pick(l);
    if (!isTransparent(code)) return { text: legend(code, current.host_layout), dim: l !== current.layer };
  }
  return { text: "", dim: false };
}

/**
 * Rough width of a string in em. Kana and kanji are square, latin is not. A
 * line of nothing but capitals and digits ("MB1", "MO1") is counted wider -
 * at the lower-case width it touched the edges of the cap - while mixed case
 * ("Esc", "Tab") keeps the size it had.
 */
function widthEm(line: string): number {
  const latin = /^[A-Z0-9]+$/.test(line) ? 0.68 : 0.58;
  let em = 0;
  for (const ch of line) em += /[　-ヿ㐀-鿿＀-｠]/.test(ch) ? 1 : latin;
  return em;
}

/**
 * An arrow along the right side of a circle, top to bottom: clockwise. The
 * anticlockwise one is the same arc flipped upside down (overlay.css). SVG
 * rather than a character: U+2938 is the right shape but draws too small and
 * too thin to read on a keycap.
 */
function arcArrow(size: number): SVGSVGElement {
  const ns = "http://www.w3.org/2000/svg";
  const svg = document.createElementNS(ns, "svg");
  svg.setAttribute("viewBox", "0 0 24 24");
  svg.setAttribute("width", String(size));
  svg.setAttribute("height", String(size));
  svg.classList.add("arrow");
  const path = document.createElementNS(ns, "path");
  path.setAttribute("d", "M 8 3.5 A 8.5 8.5 0 0 1 8 20.5 M 12 16.5 L 8 20.5 L 12 24");
  svg.append(path);
  return svg;
}

/**
 * Largest font size that still fits the legend inside the keycap.
 *
 * At a ninth of a 1080p screen a key is only about 33px across, so one fixed
 * size either wastes a single letter or overflows "Shift".
 */
function fontFor(text: string, w: number, h = w): number {
  const lines = text.split("\n");
  const widest = Math.max(0.58, ...lines.map(widthEm));
  const byWidth = (w * 0.88) / widest;
  const byHeight = Math.min(w, h) * (lines.length > 1 ? 0.4 : 0.52);
  return Math.min(byWidth, byHeight);
}

/** The keys the latest report says are down (or went down since the last). */
function down(s: StatePayload): Set<Id> {
  const ids = new Set<Id>();
  const cols = files.layout?.cols ?? 0;
  s.pressed.forEach((bits, row) => {
    for (let col = 0; col < cols; col++) {
      if (bits & (1 << col)) ids.add(`k${row * cols + col}`);
    }
  });
  // The reply carries encoder 0 only.
  if (s.knob & 1) ids.add("e0:0");
  if (s.knob & 2) ids.add("e0:1");
  return ids;
}

function linkText(): string {
  switch (current.link.kind) {
    case "up":
      return "";
    case "down":
      return "キーボード未接続";
    case "old_firmware":
      return "ファームが 0xA3 に答えない（押したキーは光らない）";
    case "too_large":
      return "マトリクスが 0xA3 の返事に収まらない";
  }
}

function build() {
  const now = performance.now();
  caps.clear();
  const { layout, keymap, problem } = files;
  if (problem || !layout || !keymap) {
    layerLabel.textContent = "";
    linkLabel.textContent = "";
    board.style.width = board.style.height = "";
    const msg = document.createElement("div");
    msg.className = "message";
    msg.textContent = problem ?? "";
    board.replaceChildren(msg);
    return;
  }
  layerLabel.textContent = `Layer ${current.layer}${current.debug_layer === null ? "" : "（固定）"}`;
  layerLabel.dataset.layer = String(current.layer);
  linkLabel.textContent = linkText();

  // Fit the whole board into whatever is left of the panel, keeping keys square.
  const header = layerLabel.parentElement as HTMLElement;
  const padding = Math.round(Math.min(panel.clientWidth, panel.clientHeight) * 0.05);
  const availW = panel.clientWidth - padding * 2;
  const availH = panel.clientHeight - padding * 2 - header.offsetHeight;
  const { bounds } = layout;
  const u = Math.min(availW / bounds.w, availH / bounds.h);
  const inset = u * 0.07;
  const key = u - inset * 2;

  board.style.width = `${u * bounds.w}px`;
  board.style.height = `${u * bounds.h}px`;

  /** A cap on `p`, widened by `grow` units (split evenly both sides). */
  const cap = (p: Place, text: string, cls: string, grow = 0) => {
    const el = document.createElement("div");
    const w = p.w * u - inset * 2 + grow * u;
    const h = p.h * u - inset * 2;
    const left = (p.x - bounds.x - grow / 2) * u + inset;
    const top = (p.y - bounds.y) * u + inset;
    el.className = cls;
    el.style.left = `${left}px`;
    el.style.top = `${top}px`;
    el.style.width = `${w}px`;
    el.style.height = `${h}px`;
    el.style.fontSize = `${fontFor(text, w, h)}px`;
    if (p.r) {
      // KLE turns about (rx, ry) on the board; CSS turns about a point in the box.
      el.style.transformOrigin = `${(p.rx - bounds.x) * u - left}px ${(p.ry - bounds.y) * u - top}px`;
      el.style.transform = `rotate(${p.r}deg)`;
    }
    el.textContent = text;
    return el;
  };

  const els = layout.keys.map((k) => {
    const cell = k.row * layout.cols + k.col;
    const { text, dim } = resolve((l) => keymap.layers[l]?.[cell]);
    const el = cap(k, text, "key" + (dim ? " dim" : ""));
    el.style.borderRadius = `${key * 0.14}px`;
    caps.set(`k${cell}`, el);
    return el;
  });

  // An encoder drawn as one key is a pill two keys wide on the key's centre,
  // CCW on the left half and CW on the right, each half lighting for the way it
  // was turned: one key wide, split in two, neither legend could be read. VIA's
  // own spelling, one key per direction, gets one arrow each.
  for (const e of layout.encoders) {
    const both = e.dir === null;
    const knob = cap(e, "", "knob", both ? 1 : 0);
    knob.style.borderRadius = `${key / 2}px`;
    const dirs = both ? ([0, 1] as const) : [e.dir as 0 | 1];
    for (const d of dirs) {
      const { text, dim } = resolve((l) => keymap.encoders[e.index]?.[l]?.[d]);
      const half = document.createElement("span");
      half.className = `half ${d === 0 ? "ccw" : "cw"}` + (dim ? " dim" : "");
      const label = document.createElement("span");
      label.textContent = text;
      label.style.fontSize = `${fontFor(text, key * 0.85) * 0.8}px`;
      half.append(arcArrow(key * 0.45), label);
      knob.append(half);
      caps.set(`e${e.index}:${d}`, half);
    }
    els.push(knob);
  }

  board.replaceChildren(...els);
  paint(now);
}

/** Flip `.pressed` on the caps; nothing else. */
function paint(now: number) {
  for (const [id, el] of caps) {
    const t = since.get(id);
    el.classList.toggle("pressed", t !== undefined);
  }
  // Redraw once more when the next short tap has been lit long enough.
  clearTimeout(expiry);
  const next = Math.min(...[...since.values()].map((t) => t + MIN_LIT_MS).filter((t) => t > now));
  if (Number.isFinite(next)) expiry = window.setTimeout(() => settle(performance.now()), next - now + 1);
}

/** Fold the latest report into `since`: new keys start, released keys go
 *  dark unless they are still inside MIN_LIT_MS. */
function settle(now: number) {
  const held = down(current);
  for (const id of held) if (!since.has(id)) since.set(id, now);
  for (const [id, t] of since) if (!held.has(id) && now - t >= MIN_LIT_MS) since.delete(id);
  paint(now);
}

/** Everything a keycap's legend depends on, besides the keymap itself. */
function face(): string {
  return [
    current.enabled,
    current.layer,
    current.active,
    current.debug_layer,
    current.host_layout,
    JSON.stringify(current.link),
  ].join();
}

function render() {
  if (!files || !current) return;
  panel.style.setProperty("--op", String(current.opacity));
  panel.classList.toggle("off", !current.enabled);
  if (!current.enabled) return;
  const f = face();
  if (f !== built) {
    built = f;
    build();
  }
  settle(performance.now());
}

async function main() {
  [files, current] = await Promise.all([getFiles(), getState()]);
  render();

  await listen<StatePayload>("state-changed", (e) => {
    current = e.payload;
    render();
  });
  await listen<Files>("files-changed", (e) => {
    files = e.payload;
    built = "";
    render();
  });
  window.addEventListener("resize", () => {
    built = "";
    render();
  });
}

main();
