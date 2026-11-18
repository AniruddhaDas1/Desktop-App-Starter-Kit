mod commands;
mod error;
mod state;

use tauri::{Emitter, Manager};

use commands::ai::{ai_cancel, ai_complete};
use commands::app::{app_get_info, app_open_external, app_open_path, app_show_in_folder};
use commands::dialog::dialog_open_file;
use commands::fs::{dnd_bless_and_resolve, dnd_resolve_paths, fs_list, fs_read, fs_stat, fs_write};
use commands::log::log_send;
use commands::menu::menu_show_window_menu;
use commands::notification::notification_show;
use commands::secrets::{secret_delete, secret_has, secret_set, secret_status};
use commands::settings::{settings_get, settings_reset, settings_set};
use commands::theme::{theme_get, theme_set};
use commands::window::{window_close, window_is_maximized, window_maximize, window_minimize};
use state::AppState;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(log::LevelFilter::Info)
                .build(),
        )
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .setup(|app| {
            // app_data_dir mirrors Electron's userData: it is always readable
            // through fs:* and holds settings.json + the secret fallback file.
            let app_data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&app_data_dir)?;
            let settings = commands::settings::load_settings(&app_data_dir);
            app.manage(AppState::new(app_data_dir, settings));

            if let Some(window) = app.get_webview_window("main") {
                // The window ships hidden so the webview never flashes white
                // before React paints the first frame.
                window.show()?;

                let handle = app.handle().clone();
                let window_handle = window.clone();
                window.on_window_event(move |event| match event {
                    tauri::WindowEvent::Resized(_) => {
                        if let Ok(is_maximized) = window_handle.is_maximized() {
                            let _ = handle.emit(
                                "window:on-maximize-change",
                                serde_json::json!({ "isMaximized": is_maximized }),
                            );
                        }
                    }
                    tauri::WindowEvent::ThemeChanged(_) => {
                        let state = handle.state::<AppState>();
                        let preference = state
                            .settings
                            .read()
                            .map(|settings| settings.theme.clone())
                            .unwrap_or_else(|_| "system".into());
                        if preference == "system" {
                            commands::theme::apply_and_emit(&handle, "system");
                        }
                    }
                    _ => {}
                });
            }

            Ok(())
        })
        .on_menu_event(|app, event| match event.id().as_ref() {
            "settings" => {
                let _ = app.emit(
                    "menu:action",
                    serde_json::json!({ "action": "settings:open" }),
                );
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![
            app_get_info,
            app_open_external,
            app_open_path,
            app_show_in_folder,
            dialog_open_file,
            log_send,
            settings_get,
            settings_set,
            settings_reset,
            secret_has,
            secret_set,
            secret_delete,
            secret_status,
            theme_get,
            theme_set,
            window_minimize,
            window_maximize,
            window_close,
            window_is_maximized,
            menu_show_window_menu,
            notification_show,
            ai_complete,
            ai_cancel,
            dnd_resolve_paths,
            dnd_bless_and_resolve,
            fs_read,
            fs_write,
            fs_list,
            fs_stat,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
