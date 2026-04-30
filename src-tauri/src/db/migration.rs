pub fn run_migrations(conn: &rusqlite::Connection) -> Result<(), Box<dyn std::error::Error>> {
    conn.pragma_update(None, "journal_mode", "wal")?;
    conn.pragma_update(None, "busy_timeout", 5000)?;
    conn.pragma_update(None, "foreign_keys", "on")?;

    let version: u32 = conn.pragma_query_value(None, "user_version", |row| row.get(0))?;

    if version < 1 {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS providers (
                id            TEXT PRIMARY KEY,
                name          TEXT NOT NULL,
                base_url      TEXT NOT NULL,
                api_key_enc   TEXT NOT NULL,
                protocol      TEXT NOT NULL DEFAULT 'anthropic',
                model_mapping TEXT,
                auth_header   TEXT NOT NULL DEFAULT 'x-api-key',
                keyword       TEXT NOT NULL,
                enabled       BOOLEAN DEFAULT TRUE,
                sort_order    INTEGER DEFAULT 0,
                created_at    TEXT DEFAULT (datetime('now')),
                updated_at    TEXT DEFAULT (datetime('now'))
            );

            CREATE TABLE IF NOT EXISTS request_logs (
                id            INTEGER PRIMARY KEY AUTOINCREMENT,
                request_id    TEXT NOT NULL,
                timestamp     TEXT NOT NULL,
                model         TEXT NOT NULL,
                provider_id   TEXT NOT NULL,
                provider_name TEXT NOT NULL,
                protocol      TEXT NOT NULL,
                upstream_url  TEXT NOT NULL,
                status_code   INTEGER,
                latency_ms    INTEGER,
                input_tokens  INTEGER DEFAULT 0,
                output_tokens INTEGER DEFAULT 0,
                error_message TEXT,
                is_streaming  BOOLEAN DEFAULT TRUE,
                created_at    TEXT DEFAULT (datetime('now'))
            );

            CREATE INDEX IF NOT EXISTS idx_logs_timestamp ON request_logs(timestamp);
            CREATE INDEX IF NOT EXISTS idx_logs_provider ON request_logs(provider_id);
            CREATE INDEX IF NOT EXISTS idx_logs_model ON request_logs(model);",
        )?;
        conn.pragma_update(None, "user_version", 1)?;
    }

    if version < 2 {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS app_settings (
                key        TEXT PRIMARY KEY,
                value      TEXT NOT NULL,
                updated_at TEXT DEFAULT (datetime('now'))
            );

            INSERT OR IGNORE INTO app_settings (key, value) VALUES ('proxy_port', '9876');
            INSERT OR IGNORE INTO app_settings (key, value) VALUES ('auto_start_proxy', 'false');
            INSERT OR IGNORE INTO app_settings (key, value) VALUES ('log_retention_days', '90');",
        )?;
        conn.pragma_update(None, "user_version", 2)?;
    }

    if version < 3 {
        conn.execute(
            "UPDATE providers
             SET enabled = 0, updated_at = datetime('now')
             WHERE enabled = 1
               AND id NOT IN (
                   SELECT id
                   FROM providers
                   WHERE enabled = 1
                   ORDER BY sort_order, created_at, id
                   LIMIT 1
               )",
            [],
        )?;
        conn.pragma_update(None, "user_version", 3)?;
    }

    let retention_days: u32 = conn
        .query_row(
            "SELECT value FROM app_settings WHERE key = 'log_retention_days'",
            [],
            |row| row.get::<_, String>(0),
        )
        .ok()
        .and_then(|value| value.parse::<u32>().ok())
        .unwrap_or(90);

    conn.execute(
        &format!(
            "DELETE FROM request_logs WHERE created_at < datetime('now', '-{} days')",
            retention_days.max(1)
        ),
        [],
    )?;

    Ok(())
}
