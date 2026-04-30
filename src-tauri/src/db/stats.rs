use serde::Serialize;
use std::sync::Arc;
use tokio::sync::Mutex;

pub type DbConn = Arc<Mutex<rusqlite::Connection>>;

#[derive(Debug, Clone, Serialize)]
pub struct DailyStats {
    pub date: String,
    pub total_requests: i64,
    pub total_errors: i64,
    pub total_input_tokens: i64,
    pub total_output_tokens: i64,
    pub avg_latency_ms: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProviderBreakdown {
    pub provider_name: String,
    pub requests: i64,
    pub input_tokens: i64,
    pub output_tokens: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ModelBreakdown {
    pub model: String,
    pub requests: i64,
    pub input_tokens: i64,
    pub output_tokens: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct StatsResult {
    pub today: DailyStats,
    pub provider_breakdown: Vec<ProviderBreakdown>,
    pub model_breakdown: Vec<ModelBreakdown>,
    pub trend: Vec<DailyStats>,
}

pub async fn get_stats(db: &DbConn) -> Result<StatsResult, String> {
    let db = db.clone();

    tokio::task::spawn_blocking(move || {
        let db = db.blocking_lock();

        let today: DailyStats = db
            .query_row(
                "SELECT DATE('now') as date, COUNT(*) as total_requests, SUM(CASE WHEN status_code >= 400 OR status_code IS NULL THEN 1 ELSE 0 END) as total_errors, SUM(input_tokens) as total_input_tokens, SUM(output_tokens) as total_output_tokens, AVG(CAST(latency_ms AS FLOAT)) as avg_latency_ms FROM request_logs WHERE timestamp >= datetime('now', 'start of day')",
                [],
                |row| {
                    Ok(DailyStats {
                        date: row.get(0)?,
                        total_requests: row.get(1)?,
                        total_errors: row.get(2)?,
                        total_input_tokens: row.get(3)?,
                        total_output_tokens: row.get(4)?,
                        avg_latency_ms: row.get(5)?,
                    })
                },
            )
            .unwrap_or(DailyStats {
                date: chrono::Utc::now().format("%Y-%m-%d").to_string(),
                total_requests: 0,
                total_errors: 0,
                total_input_tokens: 0,
                total_output_tokens: 0,
                avg_latency_ms: 0.0,
            });

        let mut stmt = db
            .prepare(
                "SELECT provider_name, COUNT(*) as requests, SUM(input_tokens), SUM(output_tokens) FROM request_logs WHERE timestamp >= datetime('now', 'start of day') GROUP BY provider_name",
            )
            .map_err(|e| e.to_string())?;
        let provider_rows = stmt
            .query_map([], |row| {
                Ok(ProviderBreakdown {
                    provider_name: row.get(0)?,
                    requests: row.get(1)?,
                    input_tokens: row.get(2)?,
                    output_tokens: row.get(3)?,
                })
            })
            .map_err(|e| e.to_string())?;
        let mut provider_breakdown = Vec::new();
        for row in provider_rows {
            provider_breakdown.push(row.map_err(|e| e.to_string())?);
        }

        let mut stmt = db
            .prepare(
                "SELECT model, COUNT(*) as requests, SUM(input_tokens), SUM(output_tokens) FROM request_logs WHERE timestamp >= datetime('now', 'start of day') GROUP BY model",
            )
            .map_err(|e| e.to_string())?;
        let model_rows = stmt
            .query_map([], |row| {
                Ok(ModelBreakdown {
                    model: row.get(0)?,
                    requests: row.get(1)?,
                    input_tokens: row.get(2)?,
                    output_tokens: row.get(3)?,
                })
            })
            .map_err(|e| e.to_string())?;
        let mut model_breakdown = Vec::new();
        for row in model_rows {
            model_breakdown.push(row.map_err(|e| e.to_string())?);
        }

        let mut stmt = db
            .prepare(
                "SELECT DATE(timestamp) as date, COUNT(*), SUM(CASE WHEN status_code >= 400 OR status_code IS NULL THEN 1 ELSE 0 END), SUM(input_tokens), SUM(output_tokens), AVG(CAST(latency_ms AS FLOAT)) FROM request_logs WHERE timestamp >= datetime('now', '-6 days', 'start of day') GROUP BY DATE(timestamp) ORDER BY date",
            )
            .map_err(|e| e.to_string())?;
        let trend_rows = stmt
            .query_map([], |row| {
                Ok(DailyStats {
                    date: row.get(0)?,
                    total_requests: row.get(1)?,
                    total_errors: row.get(2)?,
                    total_input_tokens: row.get(3)?,
                    total_output_tokens: row.get(4)?,
                    avg_latency_ms: row.get(5)?,
                })
            })
            .map_err(|e| e.to_string())?;
        let mut trend = Vec::new();
        for row in trend_rows {
            trend.push(row.map_err(|e| e.to_string())?);
        }

        Ok(StatsResult {
            today,
            provider_breakdown,
            model_breakdown,
            trend,
        })
    })
    .await
    .map_err(|e| e.to_string())?
}
