mod actions;
mod files;
pub mod hid;
pub mod keymap;
pub mod layout;
mod overlay;
mod state;
mod tray;

use tauri::{AppHandle, Manager, RunEvent};

use state::{AppState, Files, StatePayload};

// The overlay page is a renderer, so these two reads are the whole JS API.
// Everything that changes state is driven from the tray or the keyboard.

#[tauri::command]
fn get_state(app: AppHandle) -> StatePayload {
    state::snapshot(&app)
}

#[tauri::command]
fn get_files(app: AppHandle) -> Files {
    app.state::<AppState>().files.lock().unwrap().clone()
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .invoke_handler(tauri::generate_handler![get_state, get_files])
        .setup(|app| {
            // No Dock icon on macOS - this app lives in the menu bar.
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let handle = app.handle().clone();
            app.manage(AppState::new(state::load_settings(&handle)));

            overlay::configure(&handle)?;
            tray::build(&handle)?;
            files::reload(&handle);
            files::watch(&handle);
            hid::spawn(&handle);
            files::first_run(&handle);
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building hotaru overlay")
        .run(|_app, event| {
            // Nothing here closes the overlay window, so this should never
            // fire on its own - but a tray app that quits because a window
            // went away is a classic way to lose the tray icon.
            if let RunEvent::ExitRequested { code: None, api, .. } = event {
                api.prevent_exit();
            }
        });
}
