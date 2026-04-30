use crate::proxy::server;
use crate::AppState;
use serde::Serialize;
use std::sync::Arc;
use tauri::State;

#[derive(Serialize)]
pub struct ProxyStatus {
    pub running: bool,
    pub port: Option<u16>,
}

#[tauri::command]
pub async fn start_proxy(state: State<'_, Arc<AppState>>) -> Result<u16, String> {
    let mut handle = state.proxy_handle.lock().await;

    if handle.is_some() {
        return Err("Proxy is already running".to_string());
    }

    let port = *state.proxy_port.lock().await;
    let proxy_handle = server::start_proxy_server(state.inner().clone(), port).await?;

    let actual_port = proxy_handle.port;
    *handle = Some(proxy_handle);

    // Update stored port if it changed
    let mut stored_port = state.proxy_port.lock().await;
    *stored_port = actual_port;

    Ok(actual_port)
}

#[tauri::command]
pub async fn stop_proxy(state: State<'_, Arc<AppState>>) -> Result<(), String> {
    let mut handle = state.proxy_handle.lock().await;

    match handle.take() {
        Some(h) => {
            h.shutdown.abort();
            Ok(())
        }
        None => Err("Proxy is not running".to_string()),
    }
}

#[tauri::command]
pub async fn proxy_status(state: State<'_, Arc<AppState>>) -> Result<ProxyStatus, String> {
    let handle = state.proxy_handle.lock().await;
    let port = handle.as_ref().map(|h| h.port);

    Ok(ProxyStatus {
        running: port.is_some(),
        port,
    })
}
