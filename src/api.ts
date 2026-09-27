// The Rust side owns the state; this page only reads it. Everything that
// changes state is driven from the tray menu or the keyboard, so there are no
// setters here.
import { invoke } from "@tauri-apps/api/core";

export type Link =
  | { kind: "down" }
  | { kind: "up" }
  | { kind: "old_firmware"; version: number }
  | { kind: "too_large" };

export type StatePayload = {
  enabled: boolean;
  opacity: number;
  /** The layer on top, whose legends are drawn. */
  layer: number;
  /** Bit n = layer n is on. KC_TRNS falls through these. */
  active: number;
  debug_layer: number | null;
  link: Link;
  /** One bitmask per matrix row, bit = column. */
  pressed: number[];
  /** Encoder 0. Bit 0: turned CCW since the last ask, bit 1: CW. */
  knob: number;
};

/** A KLE key in key units, turned `r` degrees clockwise about (rx, ry). */
export type Place = { x: number; y: number; w: number; h: number; r: number; rx: number; ry: number };

export type LayoutKey = Place & { row: number; col: number };

/** `dir` 0 CCW, 1 CW; null is one key drawn with both halves. */
export type Encoder = Place & { index: number; dir: 0 | 1 | null };

export type Layout = {
  name: string;
  rows: number;
  cols: number;
  bounds: { x: number; y: number; w: number; h: number };
  keys: LayoutKey[];
  encoders: Encoder[];
};

export type Keymap = {
  /** layers[l][row * cols + col] */
  layers: string[][];
  /** encoders[e][l] = [ccw, cw] */
  encoders: [string, string][][];
};

/** The registered files as last read; `problem` is shown instead of the board. */
export type Files = {
  layout: Layout | null;
  keymap: Keymap | null;
  problem: string | null;
};

export const getState = () => invoke<StatePayload>("get_state");
export const getFiles = () => invoke<Files>("get_files");
