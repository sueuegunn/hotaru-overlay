//! The notification-area (macOS: menu bar) presence.
//!
//! This is the only user interface the app has. There is no settings window:
//! the overlay window is transparent, click-through and never focused, so once
//! the tray is up nothing of this app appears in the taskbar or the Dock.
//!
//! Menu items call straight into `actions`, and every one of those ends at
//! `state::publish` - which calls `refresh` below. So the checkmarks always
//! describe the real state.

use tauri::image::Image;
use tauri::menu::{
    CheckMenuItem, IsMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu,
};
use tauri::tray::{TrayIcon, TrayIconBuilder};
use std::sync::Mutex;

use tauri::{AppHandle, Manager, Wry};
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_dialog::{DialogExt, MessageDialogKind};

use crate::actions;
use crate::state::{self, HostLayout, Link, Position};

const ICON_ON: &[u8] = include_bytes!("../icons/tray-on.png");
const ICON_OFF: &[u8] = include_bytes!("../icons/tray-off.png");
const ICON_TEMPLATE: &[u8] = include_bytes!("../icons/tray-template.png");

/// Coarse enough to pick from a menu, fine enough to matter. Opacity is a
/// set-once setting, so losing the slider costs very little.
const OPACITY_STEPS: [u8; 8] = [30, 40, 50, 60, 70, 80, 90, 100];


pub struct Tray {
    icon: TrayIcon<Wry>,
    enabled: CheckMenuItem<Wry>,
    opacity: Vec<(u8, CheckMenuItem<Wry>)>,
    positions: Vec<(Position, CheckMenuItem<Wry>)>,
    host_layouts: Vec<(HostLayout, CheckMenuItem<Wry>)>,
    layer_menu: Submenu<Wry>,
    /// Rebuilt whenever the keymap's layer count changes.
    layers: Mutex<Vec<(Option<u8>, CheckMenuItem<Wry>)>>,
    autostart: CheckMenuItem<Wry>,
    /// What the tooltip last said, so key presses do not rewrite it.
    shown: Mutex<Option<(u8, Link, bool)>>,
}

pub fn build(app: &AppHandle) -> tauri::Result<()> {
    let enabled = CheckMenuItem::with_id(app, "enabled", "表示", true, false, None::<&str>)?;

    let opacity: Vec<(u8, CheckMenuItem<Wry>)> = OPACITY_STEPS
        .iter()
        .map(|pct| {
            let item = CheckMenuItem::with_id(
                app,
                format!("opacity:{pct}"),
                format!("{pct}%"),
                true,
                false,
                None::<&str>,
            )?;
            Ok((*pct, item))
        })
        .collect::<tauri::Result<_>>()?;

    let positions: Vec<(Position, CheckMenuItem<Wry>)> = Position::ALL
        .iter()
        .map(|p| {
            let item = CheckMenuItem::with_id(
                app,
                format!("position:{}", p.id()),
                p.label(),
                true,
                false,
                None::<&str>,
            )?;
            Ok((*p, item))
        })
        .collect::<tauri::Result<_>>()?;

    let host_layouts: Vec<(HostLayout, CheckMenuItem<Wry>)> = HostLayout::ALL
        .iter()
        .map(|h| {
            let item = CheckMenuItem::with_id(
                app,
                format!("host_layout:{}", h.id()),
                h.label(),
                true,
                false,
                None::<&str>,
            )?;
            Ok((*h, item))
        })
        .collect::<tauri::Result<_>>()?;

    let opacity_menu = Submenu::with_items(app, "透明度", true, &refs(&opacity))?;
    let position_menu = Submenu::with_items(app, "位置", true, &refs(&positions))?;
    let host_layout_menu = Submenu::with_items(app, "配列", true, &refs(&host_layouts))?;
    let layer_menu = Submenu::with_items(app, "レイヤー", true, &[])?;
    let pick_layout = MenuItem::with_id(app, "pick_layout", "配置ファイルを選ぶ…", true, None::<&str>)?;
    let pick_keymap = MenuItem::with_id(app, "pick_keymap", "キーマップを選ぶ…", true, None::<&str>)?;
    let reload = MenuItem::with_id(app, "reload", "ファイルを読み直す", true, None::<&str>)?;
    let autostart =
        CheckMenuItem::with_id(app, "autostart", "ログイン時に起動", true, false, None::<&str>)?;

    let menu = Menu::with_items(
        app,
        &[
            &enabled,
            &opacity_menu,
            &position_menu,
            &host_layout_menu,
            &layer_menu,
            &PredefinedMenuItem::separator(app)?,
            &pick_layout,
            &pick_keymap,
            &reload,
            &PredefinedMenuItem::separator(app)?,
            &autostart,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::quit(app, Some("終了"))?,
        ],
    )?;

    let builder = TrayIconBuilder::with_id("hotaru-overlay")
        .menu(&menu)
        .on_menu_event(on_menu_event);

    // macOS wants one flat black shape it can invert for the menu bar theme.
    // Windows wants a finished white icon, and carries the on/off state that
    // way because it has no menu bar title to put it in.
    let builder = if cfg!(target_os = "macos") {
        builder
            .icon(Image::from_bytes(ICON_TEMPLATE)?)
            .icon_as_template(true)
    } else {
        builder.icon(Image::from_bytes(ICON_ON)?)
    };

    let icon = builder.build(app)?;
    app.manage(Tray {
        icon,
        enabled,
        opacity,
        positions,
        host_layouts,
        layer_menu,
        layers: Mutex::new(Vec::new()),
        autostart,
        shown: Mutex::new(None),
    });
    refresh_autostart(app);
    refresh_layers(app);
    Ok(())
}

/// Put one item per keymap layer in the layer menu. `None` follows the
/// keyboard; `Some(n)` pins layer n, for checking legends without the
/// keyboard plugged in.
pub fn refresh_layers(app: &AppHandle) {
    let Some(tray) = app.try_state::<Tray>() else { return };
    let state = app.state::<state::AppState>();
    let count = state.files.lock().unwrap().keymap.as_ref().map_or(0, |k| k.layers.len()) as u8;
    let mut layers = tray.layers.lock().unwrap();
    if layers.len() == count as usize + 1 {
        return;
    }
    for (_, item) in layers.drain(..) {
        let _ = tray.layer_menu.remove(&item);
    }
    // A pinned layer the new keymap does not have would draw nothing.
    let mut pinned = state.debug_layer.lock().unwrap();
    if pinned.is_some_and(|n| n >= count) {
        *pinned = None;
    }
    for layer in std::iter::once(None).chain((0..count).map(Some)) {
        let (id, label) = match layer {
            None => ("layer:auto".to_string(), "自動 (キーボードから)".to_string()),
            Some(n) => (format!("layer:{n}"), format!("Layer {n} 固定")),
        };
        if let Ok(item) = CheckMenuItem::with_id(app, id, label, true, false, None::<&str>) {
            let _ = tray.layer_menu.append(&item);
            layers.push((layer, item));
        }
    }
}

fn refs<T>(items: &[(T, CheckMenuItem<Wry>)]) -> Vec<&dyn IsMenuItem<Wry>> {
    items.iter().map(|(_, item)| item as &dyn IsMenuItem<Wry>).collect()
}

/// Bring the menu and the icon in line with the current state.
pub fn refresh(app: &AppHandle) {
    let Some(tray) = app.try_state::<Tray>() else {
        return; // publish() also runs during setup, before the tray exists
    };
    let s = state::snapshot(app);

    let _ = tray.enabled.set_checked(s.enabled);
    let current = (s.opacity * 100.0).round() as u8;
    for (pct, item) in &tray.opacity {
        let _ = item.set_checked(*pct == current);
    }
    let position = app.state::<state::AppState>().settings.lock().unwrap().position;
    for (p, item) in &tray.positions {
        let _ = item.set_checked(*p == position);
    }
    for (h, item) in &tray.host_layouts {
        let _ = item.set_checked(*h == s.host_layout);
    }
    for (layer, item) in tray.layers.lock().unwrap().iter() {
        let _ = item.set_checked(*layer == s.debug_layer);
    }

    *tray.shown.lock().unwrap() = Some((s.layer, s.link, s.enabled));
    let link = match s.link {
        Link::Up => "",
        Link::Down => " / キーボード未接続",
        Link::OldFirmware(_) => " / ファームが 0xA3 に答えない",
        Link::TooLarge => " / マトリクスが大きすぎてキーボードに聞けない",
    };
    let name = app.state::<state::AppState>().files.lock().unwrap().layout.as_ref().map(|l| l.name.clone());
    let name = name.filter(|n| !n.is_empty()).map(|n| format!(" [{n}]")).unwrap_or_default();
    let _ = tray.icon.set_tooltip(Some(if s.enabled {
        format!("hotaru overlay{name} - 表示中 (Layer {}){link}", s.layer)
    } else {
        format!("hotaru overlay{name} - 非表示{link}")
    }));

    // macOS only; documented as a no-op on Windows.
    let _ = tray.icon.set_title(Some(if s.enabled {
        format!("L{}", s.layer)
    } else {
        String::new()
    }));

    if !cfg!(target_os = "macos") {
        if let Ok(image) = Image::from_bytes(if s.enabled { ICON_ON } else { ICON_OFF }) {
            let _ = tray.icon.set_icon(Some(image));
        }
    }
}

/// `refresh`, but only when what the tray shows has moved. The HID thread
/// calls this on every change, and most changes are only keys going down.
pub fn refresh_if_layer_moved(app: &AppHandle) {
    let Some(tray) = app.try_state::<Tray>() else { return };
    let s = state::snapshot(app);
    if *tray.shown.lock().unwrap() != Some((s.layer, s.link, s.enabled)) {
        refresh(app);
    }
}

/// Read the real autostart registration and mirror it into the menu. Kept out
/// of `refresh` because that runs on every layer key press and this one touches
/// the registry (Windows) or the LaunchAgent plist (macOS).
fn refresh_autostart(app: &AppHandle) {
    let Some(tray) = app.try_state::<Tray>() else { return };
    let _ = tray
        .autostart
        .set_checked(app.autolaunch().is_enabled().unwrap_or(false));
}

fn on_menu_event(app: &AppHandle, event: MenuEvent) {
    let id = event.id().as_ref();
    match id {
        "enabled" => actions::toggle_enabled(app),
        "reload" => actions::reload_files(app),
        "pick_layout" => actions::pick_layout(app),
        "pick_keymap" => actions::pick_keymap(app),
        "autostart" => toggle_autostart(app),
        _ => {
            if let Some(pct) = id.strip_prefix("opacity:").and_then(|p| p.parse::<u8>().ok()) {
                actions::set_opacity(app, f64::from(pct) / 100.0);
            } else if let Some(p) = id.strip_prefix("position:").and_then(Position::from_id) {
                actions::set_position(app, p);
            } else if let Some(h) = id.strip_prefix("host_layout:").and_then(HostLayout::from_id) {
                actions::set_host_layout(app, h);
            } else if let Some(layer) = id.strip_prefix("layer:") {
                // "layer:auto" parses to None, which is what auto means here.
                actions::set_debug_layer(app, layer.parse::<u8>().ok());
            } else {
                return;
            }
        }
    }
    // The OS flips a check item on click whether we agree or not, so put the
    // menu back to whatever the state actually became.
    refresh(app);
}

fn toggle_autostart(app: &AppHandle) {
    let manager = app.autolaunch();
    let result = match manager.is_enabled() {
        Ok(true) => manager.disable(),
        Ok(false) => manager.enable(),
        Err(e) => Err(e),
    };
    if let Err(e) = result {
        error(app, &format!("自動起動を設定できませんでした: {e}"));
    }
    refresh_autostart(app);
}

/// The only way left to say anything to the user, now that there is no window.
pub fn error(app: &AppHandle, message: &str) {
    app.dialog()
        .message(message)
        .kind(MessageDialogKind::Error)
        .title("hotaru overlay")
        .show(|_| {});
}
