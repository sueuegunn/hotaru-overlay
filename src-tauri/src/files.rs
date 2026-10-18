//! The two files the user registers, and keeping them read.
//!
//! Nothing about any keyboard is built in. The paths live in the settings
//! file, so once registered they are what the app starts with; until then the
//! overlay says what to register instead of drawing. Both files are re-read
//! whenever their mtime or size moves, so saving from VIA shows up within a
//! second.

use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::time::{Duration, SystemTime};

use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_dialog::DialogExt;

use crate::state::{self, AppState, Files};
use crate::{keymap, layout};

#[derive(Clone, Copy)]
pub enum Which {
    Layout,
    Keymap,
}

impl Which {
    fn title(self) -> &'static str {
        match self {
            Which::Layout => "配置ファイル（VIA の定義 .json）を選んでください",
            Which::Keymap => "キーマップ（VIA の Save で保存した .layout.json）を選んでください",
        }
    }
}

fn read(path: &Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|e| e.to_string())
}

/// Both files as the settings name them, or why not.
pub fn read_all(layout_file: Option<&Path>, keymap_file: Option<&Path>) -> Files {
    let problem = |text: String| Files { problem: Some(text), ..Files::default() };
    let Some(lp) = layout_file else {
        return problem(
            "配置ファイルが未登録です。\nトレイのメニュー「配置ファイルを選ぶ…」から、VIA の定義ファイル（.json）を選んでください。".into(),
        );
    };
    let layout = match read(lp).and_then(|t| layout::parse(&t)) {
        Ok(l) => l,
        Err(e) => return problem(format!("配置ファイルを読めません:\n{}\n{e}", lp.display())),
    };
    let Some(kp) = keymap_file else {
        return Files {
            problem: Some(
                "キーマップが未登録です。\nVIA の Save で保存した .layout.json を、トレイのメニュー「キーマップを選ぶ…」から選んでください。".into(),
            ),
            layout: Some(layout),
            keymap: None,
        };
    };
    match read(kp).and_then(|t| keymap::parse(&t, &layout)) {
        Ok(km) => Files { layout: Some(layout), keymap: Some(km), problem: None },
        Err(e) => Files {
            problem: Some(format!("キーマップを読めません:\n{}\n{e}", kp.display())),
            layout: Some(layout),
            keymap: None,
        },
    }
}

fn paths(app: &AppHandle) -> (Option<PathBuf>, Option<PathBuf>) {
    let s = app.state::<AppState>().settings.lock().unwrap().clone();
    (s.layout_file, s.keymap_file)
}

/// Read both files again and tell the page, the tray and the HID thread.
pub fn reload(app: &AppHandle) {
    let (lp, kp) = paths(app);
    let files = read_all(lp.as_deref(), kp.as_deref());
    let state = app.state::<AppState>();
    let moved = state.files.lock().unwrap().layout != files.layout;
    *state.files.lock().unwrap() = files.clone();
    if moved {
        state.generation.fetch_add(1, Ordering::SeqCst);
    }
    let _ = app.emit("files-changed", files);
    crate::tray::refresh_layers(app);
    state::publish(app);
}

pub fn set(app: &AppHandle, which: Which, path: PathBuf) {
    {
        let state = app.state::<AppState>();
        let mut s = state.settings.lock().unwrap();
        match which {
            Which::Layout => s.layout_file = Some(path),
            Which::Keymap => s.keymap_file = Some(path),
        }
    }
    state::persist(app);
    reload(app);
}

/// Ask for a file with the system dialog. `then_keymap` goes straight on to
/// the keymap when there is none yet - the first-run path.
pub fn pick(app: &AppHandle, which: Which, then_keymap: bool) {
    let handle = app.clone();
    app.dialog().file().set_title(which.title()).add_filter("JSON", &["json"]).pick_file(move |chosen| {
        let Some(path) = chosen.and_then(|p| p.into_path().ok()) else { return };
        set(&handle, which, path);
        if then_keymap && paths(&handle).1.is_none() {
            pick(&handle, Which::Keymap, false);
        }
    });
}

/// On a first start (nothing registered yet) open the dialogs by themselves,
/// since the overlay window cannot be clicked.
pub fn first_run(app: &AppHandle) {
    match paths(app) {
        (None, _) => pick(app, Which::Layout, true),
        (Some(_), None) => pick(app, Which::Keymap, false),
        _ => {}
    }
}

/// A file's mtime and size. The size is there for a browser's save: Chrome
/// empties the chosen file first and writes it later, and the second step
/// can keep the first step's mtime, which alone left the empty read standing.
type Stamp = Option<(SystemTime, u64)>;

fn stamps(app: &AppHandle) -> [Stamp; 2] {
    let (lp, kp) = paths(app);
    let m = |p: Option<PathBuf>| p.and_then(|p| std::fs::metadata(p).ok()).and_then(|m| Some((m.modified().ok()?, m.len())));
    [m(lp), m(kp)]
}

/// Poll both stamps once a second and reload when either moves. A new
/// registration moves them too, which only costs one extra read.
pub fn watch(app: &AppHandle) {
    let app = app.clone();
    std::thread::spawn(move || {
        let mut seen = stamps(&app);
        loop {
            std::thread::sleep(Duration::from_secs(1));
            let now = stamps(&app);
            if now != seen {
                seen = now;
                // VIA writes the file in one go, but give a slow writer a beat.
                std::thread::sleep(Duration::from_millis(100));
                reload(&app);
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nothing_registered_says_what_to_do() {
        let f = read_all(None, None);
        assert!(f.layout.is_none() && f.problem.unwrap().contains("配置ファイルを選ぶ"));
    }

    #[test]
    fn a_missing_file_is_named() {
        let f = read_all(Some(Path::new("no-such-definition.json")), None);
        assert!(f.problem.unwrap().contains("no-such-definition.json"));
    }
}
