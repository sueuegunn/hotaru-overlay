//! Everything the app displays lives here, in the Rust process.
//!
//! The overlay page is a dumb renderer: it holds no state of its own, it reads
//! once and then waits for `state-changed`. The tray menu is the same idea by
//! other means, so the two never disagree about what is showing.

use std::path::PathBuf;
use std::sync::atomic::AtomicU64;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};

use crate::hid::Reply;
use crate::keymap::Keymap;
use crate::layout::Layout;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub enabled: bool,
    /// 0.1 - 1.0. Applied to the whole overlay panel, not to the window.
    pub opacity: f64,
    /// Which ninth of the screen the overlay sits in. Missing from settings
    /// files written before it existed, hence the default.
    #[serde(default)]
    pub position: Position,
    /// Which keyboard layout the OS reads the keycodes with. Missing from
    /// settings files written before it existed, hence the default.
    #[serde(default)]
    pub host_layout: HostLayout,
    /// The VIA definition registered from the tray: where the keys are.
    #[serde(default)]
    pub layout_file: Option<PathBuf>,
    /// VIA's save file registered from the tray: what the keys type.
    #[serde(default)]
    pub keymap_file: Option<PathBuf>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            enabled: true,
            opacity: 0.85,
            position: Position::default(),
            host_layout: HostLayout::default(),
            layout_file: None,
            keymap_file: None,
        }
    }
}

/// The OS keyboard layout the legends are read in. A keycode names a key's
/// place, not its character: KC_SCLN types ";" either way, but shifted it is
/// ":" on US and "+" on JIS. The keyboard cannot tell, so the user picks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HostLayout {
    #[default]
    Us,
    Jis,
}

impl HostLayout {
    /// The order of the tray menu.
    pub const ALL: [HostLayout; 2] = [HostLayout::Us, HostLayout::Jis];

    /// Also what the overlay page receives (src/keycodes.ts's HostLayout).
    pub fn id(self) -> &'static str {
        match self {
            HostLayout::Us => "us",
            HostLayout::Jis => "jis",
        }
    }

    pub fn from_id(id: &str) -> Option<HostLayout> {
        HostLayout::ALL.into_iter().find(|h| h.id() == id)
    }

    pub fn label(self) -> &'static str {
        match self {
            HostLayout::Us => "US",
            HostLayout::Jis => "JIS",
        }
    }
}

/// The nine places the overlay can sit: the screen cut into a 3x3 grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Position {
    TopLeft,
    Top,
    TopRight,
    Left,
    #[default]
    Center,
    Right,
    BottomLeft,
    Bottom,
    BottomRight,
}

impl Position {
    /// Reading order, which is also the order of the tray menu.
    pub const ALL: [Position; 9] = [
        Position::TopLeft,
        Position::Top,
        Position::TopRight,
        Position::Left,
        Position::Center,
        Position::Right,
        Position::BottomLeft,
        Position::Bottom,
        Position::BottomRight,
    ];

    /// Column and row in the grid, each 0 (left / top) to 2 (right / bottom).
    pub fn cell(self) -> (u8, u8) {
        let i = Position::ALL.iter().position(|p| *p == self).unwrap() as u8;
        (i % 3, i / 3)
    }

    pub fn id(self) -> &'static str {
        match self {
            Position::TopLeft => "top_left",
            Position::Top => "top",
            Position::TopRight => "top_right",
            Position::Left => "left",
            Position::Center => "center",
            Position::Right => "right",
            Position::BottomLeft => "bottom_left",
            Position::Bottom => "bottom",
            Position::BottomRight => "bottom_right",
        }
    }

    pub fn from_id(id: &str) -> Option<Position> {
        Position::ALL.into_iter().find(|p| p.id() == id)
    }

    pub fn label(self) -> &'static str {
        match self {
            Position::TopLeft => "左上",
            Position::Top => "上",
            Position::TopRight => "右上",
            Position::Left => "左",
            Position::Center => "中央",
            Position::Right => "右",
            Position::BottomLeft => "左下",
            Position::Bottom => "下",
            Position::BottomRight => "右下",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(tag = "kind", content = "version", rename_all = "snake_case")]
pub enum Link {
    Down,
    Up,
    /// Plugged in, but the firmware does not answer 0xA3 (or another version).
    OldFirmware(u8),
    /// The matrix does not fit the 0xA3 reply, so the keyboard is not asked.
    TooLarge,
}

pub struct AppState {
    pub settings: Mutex<Settings>,
    /// Set from the tray menu to pin a layer while checking legends.
    pub debug_layer: Mutex<Option<u8>>,
    pub files: Mutex<Files>,
    /// Bumped whenever the layout is reloaded, so the HID thread reconnects
    /// to whatever keyboard the new one names.
    pub generation: AtomicU64,
    pub link: Mutex<Link>,
    pub board: Mutex<Option<Reply>>,
}

/// The two registered files as last read. `problem` is what the overlay says
/// instead of drawing, when either is missing or does not read.
#[derive(Debug, Clone, Default, Serialize)]
pub struct Files {
    pub layout: Option<Layout>,
    pub keymap: Option<Keymap>,
    pub problem: Option<String>,
}

impl AppState {
    pub fn new(settings: Settings) -> Self {
        Self {
            settings: Mutex::new(settings),
            debug_layer: Mutex::new(None),
            files: Mutex::new(Files::default()),
            generation: AtomicU64::new(0),
            link: Mutex::new(Link::Down),
            board: Mutex::new(None),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct StatePayload {
    pub enabled: bool,
    pub opacity: f64,
    pub host_layout: HostLayout,
    /// The layer on top - the one whose legends are drawn.
    pub layer: u8,
    /// Every layer that is on, bit n = layer n. KC_TRNS falls through these.
    pub active: u32,
    pub debug_layer: Option<u8>,
    pub link: Link,
    /// One bitmask per matrix row, bit = column: down now or since the last ask.
    pub pressed: Vec<u16>,
    /// Bit 0: the knob turned CCW since the last ask, bit 1: CW.
    pub knob: u8,
}

fn settings_path(app: &AppHandle) -> Option<PathBuf> {
    Some(app.path().app_config_dir().ok()?.join("settings.json"))
}

pub fn load_settings(app: &AppHandle) -> Settings {
    settings_path(app)
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

pub fn save_settings(app: &AppHandle, s: &Settings) {
    let Some(path) = settings_path(app) else { return };
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(text) = serde_json::to_string_pretty(s) {
        let _ = std::fs::write(path, text + "\n");
    }
}

/// QMK's own rule: the highest layer that is on wins, and the default layer
/// is always under everything.
pub fn top_layer(active: u32) -> u8 {
    if active == 0 { 0 } else { 31 - active.leading_zeros() as u8 }
}

pub fn snapshot(app: &AppHandle) -> StatePayload {
    let state = app.state::<AppState>();
    let settings = state.settings.lock().unwrap().clone();
    let debug_layer = *state.debug_layer.lock().unwrap();
    let link = *state.link.lock().unwrap();
    let board = state.board.lock().unwrap().clone();

    let (live, pressed, knob) = match board {
        Some(r) => (r.layer_state | r.default_layer_state, r.rows, r.knob),
        None => (1, Vec::new(), 0),
    };
    let active = match debug_layer {
        Some(n) => 1 | (1u32 << n),
        None => live,
    };

    StatePayload {
        enabled: settings.enabled,
        opacity: settings.opacity,
        host_layout: settings.host_layout,
        layer: top_layer(active),
        active,
        debug_layer,
        link,
        pressed,
        knob,
    }
}

/// Push the current state everywhere it is displayed. Call after every change.
pub fn publish(app: &AppHandle) {
    let _ = app.emit("state-changed", snapshot(app));
    crate::tray::refresh(app);
}

/// From the HID thread, about 60 times a second. Publishes only on a change -
/// the page would redraw the board every time otherwise.
pub fn set_board(app: &AppHandle, link: Link, board: Option<Reply>) {
    let state = app.state::<AppState>();
    let link_changed = {
        let mut l = state.link.lock().unwrap();
        std::mem::replace(&mut *l, link) != link
    };
    let board_changed = {
        let mut b = state.board.lock().unwrap();
        let changed = *b != board;
        *b = board;
        changed
    };
    if link_changed || board_changed {
        let _ = app.emit("state-changed", snapshot(app));
        // The tray shows the layer and the link, not the keys.
        crate::tray::refresh_if_layer_moved(app);
    }
}

pub fn persist(app: &AppHandle) {
    let state = app.state::<AppState>();
    let settings = state.settings.lock().unwrap().clone();
    save_settings(app, &settings);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_highest_layer_wins() {
        assert_eq!(top_layer(0), 0);
        assert_eq!(top_layer(1), 0);
        assert_eq!(top_layer(0b111), 2);
        assert_eq!(top_layer(1 << 4 | 1), 4);
    }

    #[test]
    fn positions_read_like_the_screen() {
        assert_eq!(Position::TopLeft.cell(), (0, 0));
        assert_eq!(Position::Top.cell(), (1, 0));
        assert_eq!(Position::Center.cell(), (1, 1));
        assert_eq!(Position::BottomRight.cell(), (2, 2));
        for p in Position::ALL {
            assert_eq!(Position::from_id(p.id()), Some(p));
            // The menu id and the settings file spell it the same way.
            assert_eq!(serde_json::to_string(&p).unwrap(), format!("\"{}\"", p.id()));
        }
    }

    #[test]
    fn host_layouts_spell_one_way() {
        for h in HostLayout::ALL {
            assert_eq!(HostLayout::from_id(h.id()), Some(h));
            // The menu id, the settings file and the page all spell it the same way.
            assert_eq!(serde_json::to_string(&h).unwrap(), format!("\"{}\"", h.id()));
        }
    }

    #[test]
    fn old_settings_files_still_load() {
        let s: Settings = serde_json::from_str(r#"{"enabled":false,"opacity":0.5}"#).unwrap();
        assert_eq!(s.position, Position::Center);
        assert_eq!(s.host_layout, HostLayout::Us);
        assert!(!s.enabled);
        assert!(s.layout_file.is_none() && s.keymap_file.is_none());
    }
}
