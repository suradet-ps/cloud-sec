//! Cloud Sec desktop shell (Tauri 2 backend).

pub mod commands;
pub mod state;

use state::AppState;
use tauri::Manager;

/// Start the Tauri application.
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "cloud_sec_app=info,cloud_sec_ollama=info".into()),
        )
        .init();

    tauri::Builder::default()
        .manage(AppState::new())
        .setup(|app| {
            if let Some(window) = app.get_webview_window("main") {
                if let Err(e) = window.center() {
                    tracing::error!("failed to center main window: {e}");
                }
                if let Err(e) = window.show() {
                    tracing::error!("failed to show main window: {e}");
                }
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![commands::status, commands::ask])
        .run(tauri::generate_context!())
        .expect("error while running Cloud Sec");
}
