use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;

pub type DbConn = Arc<Mutex<rusqlite::Connection>>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestLog {
    pub id: i64,
    pub request_id: String,
    pub timestamp: String,
    pub model: String,
    pub provider_id: String,
    pub provider_name: String,
    pub protocol: String,
    pub upstream_url: String,
    pub status_code: Option<i32>,
    pub latency_ms: Option<i64>,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub error_message: Option<String>,
    pub is_streaming: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogFilter {
    pub provider_id: Option<String>,
    pub model: Option<String>,
    pub status_code: Option<i32>,
    pub date_from: Option<String>,
    pub date_to: Option<String>,
    pub page: u32,
    pub page_size: u32,
}

impl Default for LogFilter {
    fn default() -> Self {
        Self {
            provider_id: None,
            model: None,
            status_code: None,
            date_from: None,
            date_to: None,
            page: 1,
            page_size: 50,
        }
    }
}

pub async fn insert_log(db: &DbConn, log: &RequestLog) -> Result<(), String> {
    let db = db.clone();
    let log = log.clone();

    tokio::task::spawn_blocking(move || {
        let db = db.blocking_lock();
        db.execute(
            "INSERT INTO request_logs (request_id, timestamp, model, provider_id, provider_name, protocol, upstream_url, status_code, latency_ms, input_tokens, output_tokens, error_message, is_streaming) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            rusqlite::params![
                log.request_id, log.timestamp, log.model, log.provider_id,
                log.provider_name, log.protocol, log.upstream_url, log.status_code,
                log.latency_ms, log.input_tokens, log.output_tokens, log.error_message,
                log.is_streaming,
            ],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())?
}

const LOG_COLUMNS: &str = "id, request_id, timestamp, model, provider_id, provider_name, protocol, upstream_url, status_code, latency_ms, input_tokens, output_tokens, error_message, is_streaming";

fn build_where_clause(filter: &LogFilter) -> (String, Vec<Box<dyn rusqlite::types::ToSql>>) {
    let mut where_clauses = Vec::new();
    let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

    if let Some(ref pid) = filter.provider_id {
        where_clauses.push(format!("provider_id = ?{}", params.len() + 1));
        params.push(Box::new(pid.clone()));
    }
    if let Some(ref m) = filter.model {
        where_clauses.push(format!("model LIKE ?{}", params.len() + 1));
        params.push(Box::new(format!("%{}%", m)));
    }
    if let Some(sc) = filter.status_code {
        where_clauses.push(format!("status_code = ?{}", params.len() + 1));
        params.push(Box::new(sc));
    }
    if let Some(ref df) = filter.date_from {
        where_clauses.push(format!("timestamp >= ?{}", params.len() + 1));
        params.push(Box::new(df.clone()));
    }
    if let Some(ref dt) = filter.date_to {
        where_clauses.push(format!("timestamp <= ?{}", params.len() + 1));
        params.push(Box::new(dt.clone()));
    }

    let where_sql = if where_clauses.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", where_clauses.join(" AND "))
    };

    (where_sql, params)
}

fn row_to_log(row: &rusqlite::Row<'_>) -> rusqlite::Result<RequestLog> {
    Ok(RequestLog {
        id: row.get(0)?,
        request_id: row.get(1)?,
        timestamp: row.get(2)?,
        model: row.get(3)?,
        provider_id: row.get(4)?,
        provider_name: row.get(5)?,
        protocol: row.get(6)?,
        upstream_url: row.get(7)?,
        status_code: row.get(8)?,
        latency_ms: row.get(9)?,
        input_tokens: row.get(10)?,
        output_tokens: row.get(11)?,
        error_message: row.get(12)?,
        is_streaming: row.get(13)?,
    })
}

pub async fn query_logs(db: &DbConn, filter: &LogFilter) -> Result<(Vec<RequestLog>, u32), String> {
    let db = db.clone();
    let filter = filter.clone();

    tokio::task::spawn_blocking(move || {
        let db = db.blocking_lock();
        let (where_sql, params) = build_where_clause(&filter);

        let count_sql = format!("SELECT COUNT(*) FROM request_logs {}", where_sql);
        let total: u32 = db
            .query_row(
                &count_sql,
                rusqlite::params_from_iter(params.iter().map(|p| p.as_ref())),
                |row| row.get(0),
            )
            .unwrap_or(0);

        let offset = (filter.page.saturating_sub(1)) * filter.page_size;
        let query_sql = format!(
            "SELECT {} FROM request_logs {} ORDER BY id DESC LIMIT ?{} OFFSET ?{}",
            LOG_COLUMNS,
            where_sql,
            params.len() + 1,
            params.len() + 2
        );

        let mut all_params: Vec<Box<dyn rusqlite::types::ToSql>> = params;
        all_params.push(Box::new(filter.page_size));
        all_params.push(Box::new(offset));

        let mut stmt = db.prepare(&query_sql).map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(
                rusqlite::params_from_iter(all_params.iter().map(|p| p.as_ref())),
                row_to_log,
            )
            .map_err(|e| e.to_string())?;

        let mut logs = Vec::new();
        for row in rows {
            logs.push(row.map_err(|e| e.to_string())?);
        }

        Ok((logs, total))
    })
    .await
    .map_err(|e| e.to_string())?
}

pub async fn query_logs_all(db: &DbConn, filter: &LogFilter) -> Result<Vec<RequestLog>, String> {
    let db = db.clone();
    let filter = filter.clone();

    tokio::task::spawn_blocking(move || {
        let db = db.blocking_lock();
        let (where_sql, params) = build_where_clause(&filter);

        let query_sql = format!(
            "SELECT {} FROM request_logs {} ORDER BY id DESC",
            LOG_COLUMNS, where_sql
        );

        let mut stmt = db.prepare(&query_sql).map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(
                rusqlite::params_from_iter(params.iter().map(|p| p.as_ref())),
                row_to_log,
            )
            .map_err(|e| e.to_string())?;

        let mut logs = Vec::new();
        for row in rows {
            logs.push(row.map_err(|e| e.to_string())?);
        }

        Ok(logs)
    })
    .await
    .map_err(|e| e.to_string())?
}

pub async fn cleanup_old_logs(db: &DbConn, retention_days: u32) -> Result<(), String> {
    let db = db.clone();
    let retention_days = retention_days.max(1);

    tokio::task::spawn_blocking(move || {
        let db = db.blocking_lock();
        db.execute(
            &format!(
                "DELETE FROM request_logs WHERE created_at < datetime('now', '-{} days')",
                retention_days
            ),
            [],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())?
}
