//! Everything that changes what the overlay shows, from the tray menu.
//!
//! Every one of these ends at `state::publish`, which also refreshes the tray,
//! so the menu checkmarks always describe the real state.

use tauri::{AppHandle, Manager};

use crate::files::{self, Which};
use crate::overlay;
use crate::state::{self, AppState, HostLayout, Position};

pub fn set_enabled(app: &AppHandle, enabled: bool) {
    app.state::<AppState>().settings.lock().unwrap().enabled = enabled;
    state::persist(app);
    state::publish(app);
}

pub fn toggle_enabled(app: &AppHandle) {
    let enabled = app.state::<AppState>().settings.lock().unwrap().enabled;
    set_enabled(app, !enabled);
}

pub fn set_opacity(app: &AppHandle, opacity: f64) {
    app.state::<AppState>().settings.lock().unwrap().opacity = opacity.clamp(0.1, 1.0);
    state::persist(app);
    state::publish(app);
}

pub fn set_position(app: &AppHandle, position: Position) {
    app.state::<AppState>().settings.lock().unwrap().position = position;
    state::persist(app);
    if let Some(win) = app.get_webview_window("overlay") {
        let _ = overlay::place(&win, position);
    }
    state::publish(app);
}

pub fn set_host_layout(app: &AppHandle, host_layout: HostLayout) {
    app.state::<AppState>().settings.lock().unwrap().host_layout = host_layout;
    state::persist(app);
    state::publish(app);
}

pub fn set_debug_layer(app: &AppHandle, layer: Option<u8>) {
    *app.state::<AppState>().debug_layer.lock().unwrap() = layer;
    state::publish(app);
}

pub fn reload_files(app: &AppHandle) {
    files::reload(app);
}

pub fn pick_layout(app: &AppHandle) {
    files::pick(app, Which::Layout, true);
}

pub fn pick_keymap(app: &AppHandle) {
    files::pick(app, Which::Keymap, false);
}
