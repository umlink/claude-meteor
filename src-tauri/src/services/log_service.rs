use crate::db::logs::{DbConn, LogFilter, RequestLog};

pub async fn query_paginated(
    db: &DbConn,
    filter: &LogFilter,
) -> Result<(Vec<RequestLog>, u32), String> {
    crate::db::logs::query_logs(db, filter).await
}

pub async fn query_all(db: &DbConn, filter: &LogFilter) -> Result<Vec<RequestLog>, String> {
    crate::db::logs::query_logs_all(db, filter).await
}

pub fn format_json(logs: &[RequestLog]) -> Result<serde_json::Value, String> {
    let filename = format!(
        "claude-dynamic-meteor-logs-{}.json",
        chrono::Local::now().format("%Y%m%d-%H%M%S")
    );
    Ok(serde_json::json!({
        "filename": filename,
        "mime_type": "application/json",
        "content": serde_json::to_string_pretty(logs).map_err(|e| e.to_string())?
    }))
}

pub fn format_csv(logs: &[RequestLog]) -> Result<serde_json::Value, String> {
    let mut lines = vec![
        "id,request_id,timestamp,model,provider_id,provider_name,protocol,upstream_url,status_code,latency_ms,input_tokens,output_tokens,error_message,is_streaming".to_string()
    ];

    for log in logs {
        let escape = |value: &str| format!("\"{}\"", value.replace('"', "\"\""));
        lines.push(format!(
            "{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
            log.id,
            escape(&log.request_id),
            escape(&log.timestamp),
            escape(&log.model),
            escape(&log.provider_id),
            escape(&log.provider_name),
            escape(&log.protocol),
            escape(&log.upstream_url),
            log.status_code.map(|v| v.to_string()).unwrap_or_default(),
            log.latency_ms.map(|v| v.to_string()).unwrap_or_default(),
            log.input_tokens,
            log.output_tokens,
            escape(log.error_message.as_deref().unwrap_or("")),
            log.is_streaming
        ));
    }

    let filename = format!(
        "claude-dynamic-meteor-logs-{}.csv",
        chrono::Local::now().format("%Y%m%d-%H%M%S")
    );
    Ok(serde_json::json!({
        "filename": filename,
        "mime_type": "text/csv;charset=utf-8",
        "content": lines.join("\n")
    }))
}
