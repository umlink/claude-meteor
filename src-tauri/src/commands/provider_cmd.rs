use crate::config::provider::Provider;
use crate::config::store;
use crate::services::provider_service;
use crate::AppState;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub async fn list_providers(state: State<'_, Arc<AppState>>) -> Result<Vec<Provider>, String> {
    store::list_providers(&state.db).await
}

#[tauri::command]
pub async fn create_provider(
    state: State<'_, Arc<AppState>>,
    name: String,
    base_url: String,
    api_key: String,
    protocol: String,
    model_mapping: Option<String>,
    auth_header: String,
    keyword: String,
    enabled: bool,
) -> Result<Provider, String> {
    provider_service::create(
        &state.db,
        name,
        base_url,
        api_key,
        protocol,
        model_mapping,
        auth_header,
        keyword,
        enabled,
    )
    .await
}

#[tauri::command]
pub async fn update_provider(
    state: State<'_, Arc<AppState>>,
    id: String,
    name: String,
    base_url: String,
    api_key: Option<String>,
    protocol: String,
    model_mapping: Option<String>,
    auth_header: String,
    keyword: String,
    enabled: bool,
) -> Result<(), String> {
    provider_service::update(
        &state.db,
        id,
        name,
        base_url,
        api_key,
        protocol,
        model_mapping,
        auth_header,
        keyword,
        enabled,
    )
    .await
}

#[tauri::command]
pub async fn delete_provider(state: State<'_, Arc<AppState>>, id: String) -> Result<(), String> {
    provider_service::delete(&state.db, id).await
}
