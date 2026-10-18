// QMK / VIA keycode string -> what to print on the key.
//
// The keymap file may hold either form: a real keycode ("KC_TAB", "MO(1)"),
// which is what VIA saves, or a plain legend ("Tab") typed by hand.
// Anything this module does not recognise is passed through unchanged, so both
// work and hand edits never have to look like C.

const BASE: Record<string, string> = {
  KC_NO: "", XXXXXXX: "", KC_TRANSPARENT: "", KC_TRNS: "", _______: "",

  KC_ENTER: "Ent", KC_ENT: "Ent", KC_ESCAPE: "Esc", KC_ESC: "Esc",
  KC_BACKSPACE: "BS", KC_BSPC: "BS", KC_TAB: "Tab", KC_SPACE: "Spc",
  KC_SPC: "Spc", KC_DELETE: "Del", KC_DEL: "Del", KC_INSERT: "Ins",
  KC_INS: "Ins", KC_CAPS: "Caps", KC_CAPS_LOCK: "Caps",

  KC_MINUS: "-", KC_MINS: "-", KC_EQUAL: "=", KC_EQL: "=",
  KC_LEFT_BRACKET: "[", KC_LBRC: "[", KC_RIGHT_BRACKET: "]", KC_RBRC: "]",
  KC_BACKSLASH: "\\", KC_BSLS: "\\", KC_SEMICOLON: ";", KC_SCLN: ";",
  KC_QUOTE: "'", KC_QUOT: "'", KC_GRAVE: "`", KC_GRV: "`",
  KC_COMMA: ",", KC_COMM: ",", KC_DOT: ".", KC_SLASH: "/", KC_SLSH: "/",

  KC_LEFT_CTRL: "Ctl", KC_LCTL: "Ctl", KC_RIGHT_CTRL: "Ctl", KC_RCTL: "Ctl",
  KC_LEFT_SHIFT: "Sft", KC_LSFT: "Sft", KC_RIGHT_SHIFT: "Sft",
  KC_RSFT: "Sft", KC_LEFT_ALT: "Alt", KC_LALT: "Alt", KC_LOPT: "Alt",
  KC_RIGHT_ALT: "AGr", KC_RALT: "AGr", KC_ALGR: "AGr", KC_ROPT: "AGr",
  KC_LEFT_GUI: "Win", KC_LGUI: "Win", KC_LCMD: "Cmd", KC_LWIN: "Win",
  KC_RIGHT_GUI: "Win", KC_RGUI: "Win", KC_APP: "App", KC_APPLICATION: "App",

  KC_LEFT: "←", KC_RIGHT: "→", KC_RGHT: "→", KC_UP: "↑", KC_DOWN: "↓",
  KC_HOME: "Home", KC_END: "End", KC_PAGE_UP: "PgUp", KC_PGUP: "PgUp",
  KC_PAGE_DOWN: "PgDn", KC_PGDN: "PgDn",
  KC_PRINT_SCREEN: "PrSc", KC_PSCR: "PrSc", KC_PAUSE: "Paus",
  KC_PAUS: "Paus", KC_SCROLL_LOCK: "ScLk", KC_SCRL: "ScLk",

  // JIS / IME keys. The ろ key types \ and, shifted, _ - shown as the _.
  KC_INTERNATIONAL_1: "_", KC_INT1: "_", KC_RO: "_",
  KC_INTERNATIONAL_2: "かな", KC_INT2: "かな", KC_KANA: "かな",
  KC_INTERNATIONAL_3: "¥", KC_INT3: "¥", KC_JYEN: "¥",
  KC_INTERNATIONAL_4: "変換", KC_INT4: "変換", KC_HENK: "変換",
  KC_INTERNATIONAL_5: "無変", KC_INT5: "無変", KC_MHEN: "無変",
  KC_LANGUAGE_1: "英数", KC_LNG1: "英数", KC_LANGUAGE_2: "かな", KC_LNG2: "かな",
  KC_NONUS_HASH: "#", KC_NUHS: "#", KC_NONUS_BACKSLASH: "\\", KC_NUBS: "\\",

  KC_AUDIO_MUTE: "Mute", KC_MUTE: "Mute",
  KC_AUDIO_VOL_UP: "Vol+", KC_VOLU: "Vol+",
  KC_AUDIO_VOL_DOWN: "Vol-", KC_VOLD: "Vol-",
  KC_MEDIA_PLAY_PAUSE: "▶︎", KC_MPLY: "▶︎",
  KC_MEDIA_NEXT_TRACK: "⏭", KC_MNXT: "⏭",
  KC_MEDIA_PREV_TRACK: "⏮", KC_MPRV: "⏮",

  KC_NUM_LOCK: "NmLk", KC_NUM: "NmLk",
  KC_KP_SLASH: "/", KC_PSLS: "/", KC_KP_ASTERISK: "*", KC_PAST: "*",
  KC_KP_MINUS: "-", KC_PMNS: "-", KC_KP_PLUS: "+", KC_PPLS: "+",
  KC_KP_ENTER: "Ent", KC_PENT: "Ent", KC_KP_DOT: ".", KC_PDOT: ".",

  // Mouse keys, under both VIA's older names (what VIA's save files hold)
  // and QMK's current ones.
  KC_MS_BTN1: "MB1", KC_BTN1: "MB1", MS_BTN1: "MB1",
  KC_MS_BTN2: "MB2", KC_BTN2: "MB2", MS_BTN2: "MB2",
  KC_MS_BTN3: "MB3", KC_BTN3: "MB3", MS_BTN3: "MB3",
  KC_MS_BTN4: "MB4", KC_BTN4: "MB4", MS_BTN4: "MB4",
  KC_MS_BTN5: "MB5", KC_BTN5: "MB5", MS_BTN5: "MB5",
  KC_MS_WH_UP: "Wh↑", KC_WH_U: "Wh↑", MS_WHLU: "Wh↑",
  KC_MS_WH_DOWN: "Wh↓", KC_WH_D: "Wh↓", MS_WHLD: "Wh↓",
  KC_MS_WH_LEFT: "Wh←", KC_WH_L: "Wh←", MS_WHLL: "Wh←",
  KC_MS_WH_RIGHT: "Wh→", KC_WH_R: "Wh→", MS_WHLR: "Wh→",
  KC_MS_UP: "M↑", KC_MS_U: "M↑", MS_UP: "M↑",
  KC_MS_DOWN: "M↓", KC_MS_D: "M↓", MS_DOWN: "M↓",
  KC_MS_LEFT: "M←", KC_MS_L: "M←", MS_LEFT: "M←",
  KC_MS_RIGHT: "M→", KC_MS_R: "M→", MS_RGHT: "M→",

  QK_BOOTLOADER: "Boot", QK_BOOT: "Boot", RESET: "Boot",
  QK_REBOOT: "Reboot", QK_CLEAR_EEPROM: "EEclr", EE_CLR: "EEclr",
};

/**
 * The OS keyboard layout the keycodes are read in (src-tauri's HostLayout).
 * A keycode names a key's place, so the same KC_SCLN shifted is ":" on US and
 * "+" on JIS.
 */
export type HostLayout = "us" | "jis";

// Where JIS prints something else than BASE: the keys right of P, L and 0.
// The ろ key types \ (BASE shows its shifted _, which US cannot type).
const JIS_BASE: Record<string, string> = {
  KC_EQUAL: "^", KC_EQL: "^",
  KC_LEFT_BRACKET: "@", KC_LBRC: "@", KC_RIGHT_BRACKET: "[", KC_RBRC: "[",
  KC_BACKSLASH: "]", KC_BSLS: "]", KC_NONUS_HASH: "]", KC_NUHS: "]",
  KC_QUOTE: ":", KC_QUOT: ":", KC_GRAVE: "半全", KC_GRV: "半全",
  KC_INTERNATIONAL_1: "\\", KC_INT1: "\\", KC_RO: "\\",
};

// Shifted symbols, so LSFT(KC_9) prints "(" rather than "S-9".
const SHIFTED: Record<HostLayout, Record<string, string>> = {
  us: {
    KC_1: "!", KC_2: "@", KC_3: "#", KC_4: "$", KC_5: "%",
    KC_6: "^", KC_7: "&", KC_8: "*", KC_9: "(", KC_0: ")",
    KC_MINS: "_", KC_MINUS: "_", KC_EQL: "+", KC_EQUAL: "+",
    KC_LBRC: "{", KC_RBRC: "}", KC_BSLS: "|", KC_SCLN: ":",
    KC_QUOT: '"', KC_GRV: "~", KC_COMM: "<", KC_DOT: ">", KC_SLSH: "?",
  },
  // Shift+0 types nothing on JIS, so KC_0 is left to print "⇧0".
  jis: {
    KC_1: "!", KC_2: '"', KC_3: "#", KC_4: "$", KC_5: "%",
    KC_6: "&", KC_7: "'", KC_8: "(", KC_9: ")",
    KC_MINS: "=", KC_MINUS: "=", KC_EQL: "~", KC_EQUAL: "~",
    KC_LBRC: "`", KC_RBRC: "{", KC_BSLS: "}", KC_NUHS: "}",
    KC_SCLN: "+", KC_QUOT: "*", KC_COMM: "<", KC_DOT: ">", KC_SLSH: "?",
    KC_INT1: "_", KC_RO: "_", KC_INT3: "|", KC_JYEN: "|",
  },
};

const MOD_PREFIX: Record<string, string> = {
  LSFT: "⇧", RSFT: "⇧", LCTL: "⌃", RCTL: "⌃",
  LALT: "⌥", RALT: "⌥", LGUI: "⌘", RGUI: "⌘",
  S: "⇧", C: "⌃", A: "⌥", G: "⌘", LSG: "⇧⌘", LCA: "⌃⌥", LCAG: "⌃⌥⌘",
  HYPR: "Hyper", MEH: "Meh",
};

const MOD_TAP: Record<string, string> = {
  LSFT_T: "⇧", RSFT_T: "⇧", LCTL_T: "⌃", RCTL_T: "⌃", CTL_T: "⌃",
  LALT_T: "⌥", RALT_T: "⌥", ALT_T: "⌥", LGUI_T: "⌘", RGUI_T: "⌘",
  GUI_T: "⌘", SFT_T: "⇧", MEH_T: "Meh", HYPR_T: "Hyper", ALL_T: "Hyper",
};

/** Split "MO(1)" into ["MO", ["1"]]; returns null for a plain keycode. */
function call(code: string): [string, string[]] | null {
  const m = /^([A-Z0-9_]+)\((.*)\)$/.exec(code);
  if (!m) return null;
  const args: string[] = [];
  let buf = "";
  let depth = 0;
  for (const ch of m[2]) {
    if (ch === "(") depth++;
    else if (ch === ")") depth--;
    if (ch === "," && depth === 0) {
      args.push(buf.trim());
      buf = "";
    } else {
      buf += ch;
    }
  }
  if (buf.trim()) args.push(buf.trim());
  return [m[1], args];
}

/**
 * Legend for one keycode. A "\n" in the result is a deliberate second line
 * (hold action on top, tap action below) - the overlay renders it as such.
 */
export function legend(code: string | undefined | null, host: HostLayout = "us"): string {
  if (code === undefined || code === null) return "";
  const raw = String(code).trim();
  if (raw === "" || raw === "-1") return "";
  if (host === "jis" && raw in JIS_BASE) return JIS_BASE[raw];
  if (raw in BASE) return BASE[raw];

  const c = call(raw);
  if (c) {
    const [fn, args] = c;
    switch (fn) {
      case "MO":
        return `MO${args[0]}`;
      case "TO":
        return `TO${args[0]}`;
      case "TG":
        return `TG${args[0]}`;
      case "TT":
        return `TT${args[0]}`;
      case "DF":
        return `DF${args[0]}`;
      case "OSL":
        return `OSL${args[0]}`;
      case "LT":
        return `L${args[0]}\n${legend(args[1], host)}`;
      case "LM":
        return `L${args[0]}\n${MOD_PREFIX[args[1]] ?? args[1]}`;
      case "MT":
        return `${MOD_PREFIX[args[0]] ?? args[0]}\n${legend(args[1], host)}`;
      case "OSM":
        return `${MOD_PREFIX[args[0]] ?? args[0]}\nOS`;
      case "TD":
        return `TD${args[0]}`;
      case "MACRO":
        return `M${args[0]}`;
    }
    if (fn in MOD_TAP) return `${MOD_TAP[fn]}\n${legend(args[0], host)}`;
    if (fn in MOD_PREFIX) {
      const inner = args[0];
      const shifted = MOD_PREFIX[fn] === "⇧" ? SHIFTED[host][inner] : undefined;
      return shifted ?? `${MOD_PREFIX[fn]}${legend(inner, host)}`;
    }
    return raw;
  }

  if (/^M\d+$/.test(raw)) return raw; // Vial macro
  if (/^KC_F\d+$/.test(raw)) return raw.slice(3);
  if (/^KC_[A-Z0-9]$/.test(raw)) return raw.slice(3);
  if (raw.startsWith("KC_")) return raw.slice(3);
  return raw; // already a hand written legend
}

/** True when the cell should fall through to the layer below. */
export function isTransparent(code: string | undefined | null): boolean {
  if (code === undefined || code === null) return true;
  const raw = String(code).trim();
  return raw === "" || raw === "KC_TRNS" || raw === "KC_TRANSPARENT" || raw === "_______";
}
