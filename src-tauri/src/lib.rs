mod adapter;
mod claude;
mod commands;
mod config;
mod db;
mod proxy;
mod services;
mod tray;

use std::sync::Arc;
use tauri::Manager;
use tokio::sync::Mutex;

pub struct AppState {
    pub db: Arc<Mutex<rusqlite::Connection>>,
    pub proxy_handle: Arc<Mutex<Option<proxy::server::ProxyHandle>>>,
    pub proxy_port: Arc<Mutex<u16>>,
    pub http_client: reqwest::Client,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--minimized"]),
        ))
        .setup(|app| {
            let db_path = dirs::data_dir()
                .unwrap_or_else(|| std::path::PathBuf::from("."))
                .join("claude-dynamic-meteor")
                .join("meteor.db");

            std::fs::create_dir_all(db_path.parent().unwrap())?;

            let conn = rusqlite::Connection::open(&db_path)?;
            db::migration::run_migrations(&conn)?;
            let settings = config::app_settings::get_app_settings_sync(&conn)
                .unwrap_or_else(|_| config::app_settings::AppSettings::default());

            let state = Arc::new(AppState {
                db: Arc::new(Mutex::new(conn)),
                proxy_handle: Arc::new(Mutex::new(None)),
                proxy_port: Arc::new(Mutex::new(settings.proxy_port)),
                http_client: reqwest::Client::builder()
                    .timeout(std::time::Duration::from_secs(120))
                    .build()
                    .expect("failed to build http client"),
            });

            app.manage(state.clone());

            // Create system tray
            tray::create_tray(app.handle())?;

            if settings.auto_start_proxy {
                let state = state.clone();
                tauri::async_runtime::spawn(async move {
                    match proxy::server::start_proxy_server(state.clone(), settings.proxy_port)
                        .await
                    {
                        Ok(proxy_handle) => {
                            let actual_port = proxy_handle.port;
                            *state.proxy_port.lock().await = actual_port;
                            *state.proxy_handle.lock().await = Some(proxy_handle);
                        }
                        Err(error) => {
                            tracing::error!("Failed to auto-start proxy: {}", error);
                        }
                    }
                });
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::server_cmd::start_proxy,
            commands::server_cmd::stop_proxy,
            commands::server_cmd::proxy_status,
            commands::provider_cmd::list_providers,
            commands::provider_cmd::create_provider,
            commands::provider_cmd::update_provider,
            commands::provider_cmd::delete_provider,
            commands::log_cmd::get_logs,
            commands::log_cmd::export_logs,
            commands::stats_cmd::get_stats,
            commands::claude_cmd::inject_claude_config,
            commands::claude_cmd::revert_claude_config,
            commands::claude_cmd::check_claude_config,
            commands::settings_cmd::get_app_settings,
            commands::settings_cmd::update_app_settings,
            commands::autostart_cmd::is_autostart_enabled,
            commands::autostart_cmd::enable_autostart,
            commands::autostart_cmd::disable_autostart,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
