use crate::db::logs;
use crate::services::log_service;
use crate::AppState;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub async fn get_logs(
    state: State<'_, Arc<AppState>>,
    provider_id: Option<String>,
    model: Option<String>,
    status_code: Option<i32>,
    date_from: Option<String>,
    date_to: Option<String>,
    page: Option<u32>,
    page_size: Option<u32>,
) -> Result<serde_json::Value, String> {
    let filter = logs::LogFilter {
        provider_id,
        model,
        status_code,
        date_from,
        date_to,
        page: page.unwrap_or(1),
        page_size: page_size.unwrap_or(50),
    };

    let (logs_list, total) = log_service::query_paginated(&state.db, &filter).await?;

    Ok(serde_json::json!({
        "logs": logs_list,
        "total": total,
        "page": filter.page,
        "page_size": filter.page_size
    }))
}

#[tauri::command]
pub async fn export_logs(
    state: State<'_, Arc<AppState>>,
    format: String,
    provider_id: Option<String>,
    model: Option<String>,
    status_code: Option<i32>,
    date_from: Option<String>,
    date_to: Option<String>,
) -> Result<serde_json::Value, String> {
    let filter = logs::LogFilter {
        provider_id,
        model,
        status_code,
        date_from,
        date_to,
        page: 1,
        page_size: 50,
    };

    let exported = log_service::query_all(&state.db, &filter).await?;

    match format.as_str() {
        "json" => log_service::format_json(&exported),
        "csv" => log_service::format_csv(&exported),
        _ => Err("Unsupported export format".to_string()),
    }
}
