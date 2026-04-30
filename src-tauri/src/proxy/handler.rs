use crate::services::proxy_service;
use crate::AppState;
use axum::body::Body;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use serde_json::Value;
use std::sync::Arc;
use std::time::Instant;

// Security logging policy:
// - NEVER log API key values, tokens, or plaintext secrets
// - MAY log API key length, emptiness, or encrypted prefix (debug level only)
// - MAY log provider name, model, upstream URL, status codes, latency
// - All auth-related debug output must be behind `tracing::debug!`, not `info!`

pub async fn handle_messages(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Response {
    let start = Instant::now();
    let request_id = uuid::Uuid::new_v4().to_string();

    let mut request_json: Value = match serde_json::from_slice(&body) {
        Ok(v) => v,
        Err(e) => return anthropic_error(400, &format!("Invalid request body: {}", e)),
    };

    let resolved =
        match proxy_service::resolve_request(&state.db, &mut request_json, body.as_ref()).await {
            Ok(resolved) => resolved,
            Err(error) if error == "No enabled provider configured" => {
                return anthropic_error(400, &error)
            }
            Err(error) if error.starts_with("Failed to serialize") => {
                return anthropic_error(500, &error)
            }
            Err(error) if error.starts_with("Failed to load providers") => {
                return anthropic_error(500, &error)
            }
            Err(error) if error.contains("Request conversion failed") => {
                return anthropic_error(500, &error)
            }
            Err(error) => {
                return anthropic_error(500, &format!("Request preparation failed: {}", error))
            }
        };
    let log_context = proxy_service::make_log_context(request_id, &resolved);

    let api_key = crate::config::store::decrypt_api_key(&resolved.provider.api_key_enc);
    tracing::debug!(
        "Provider: {}, API Key length: {}, Is empty: {}",
        resolved.provider.name,
        api_key.len(),
        api_key.is_empty()
    );
    tracing::debug!("Upstream URL: {}", resolved.upstream_url);

    let req_builder = state
        .http_client
        .post(&resolved.upstream_url)
        .header("Content-Type", "application/json");
    let mut req_builder = proxy_service::apply_auth_headers(
        req_builder,
        &resolved.provider,
        &api_key,
        resolved.is_openai,
    );

    if let Some(av) = headers.get("anthropic-version") {
        req_builder = req_builder.header("anthropic-version", av);
    }

    let resp = match req_builder.body(resolved.upstream_body).send().await {
        Ok(r) => r,
        Err(e) => {
            proxy_service::log_upstream_send_error(&state.db, &log_context, start, &e).await;
            return anthropic_error(502, &format!("Upstream unreachable: {}", e));
        }
    };

    let status = resp.status();
    let status_code = status.as_u16() as i32;

    if resolved.is_streaming && status.is_success() {
        let stream = resp.bytes_stream();
        let monitored = if resolved.is_openai {
            crate::adapter::openai::convert_sse_stream(
                stream,
                &resolved.model,
                resolved.provider.model_mapping.as_deref(),
            )
        } else {
            crate::adapter::anthropic::monitor_sse_stream(stream)
        };

        let crate::adapter::MonitoredSseStream {
            stream: response_stream,
            summary,
        } = monitored;

        proxy_service::spawn_stream_logger(&state.db, summary, log_context, status_code, start);

        Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", "text/event-stream")
            .header("Cache-Control", "no-cache")
            .header("Connection", "keep-alive")
            .body(Body::from_stream(response_stream))
            .unwrap()
    } else {
        handle_non_streaming(
            &state.db,
            resp,
            resolved.is_openai,
            status_code,
            &log_context,
            &resolved.provider,
            start,
        )
        .await
    }
}

async fn handle_non_streaming(
    db: &crate::db::logs::DbConn,
    resp: reqwest::Response,
    is_openai: bool,
    status_code: i32,
    log_context: &proxy_service::RequestLogContext,
    provider: &crate::config::provider::Provider,
    start: Instant,
) -> Response {
    let resp_bytes = match resp.bytes().await {
        Ok(b) => b,
        Err(e) => return anthropic_error(502, &format!("Failed to read response: {}", e)),
    };

    let latency = start.elapsed().as_millis() as i64;

    if is_openai
        && StatusCode::from_u16(status_code as u16)
            .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)
            .is_success()
    {
        let json: Value = match serde_json::from_slice(&resp_bytes) {
            Ok(v) => v,
            Err(_) => return anthropic_error(500, "Failed to parse upstream response"),
        };

        let converted = crate::adapter::openai::convert_response(
            &json,
            &log_context.model,
            provider.model_mapping.as_deref(),
        );
        let resp_body = serde_json::to_vec(&converted).unwrap_or(resp_bytes.to_vec());

        let input_tokens = converted
            .get("usage")
            .and_then(|u| u.get("input_tokens"))
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        let output_tokens = converted
            .get("usage")
            .and_then(|u| u.get("output_tokens"))
            .and_then(|v| v.as_i64())
            .unwrap_or(0);

        let _ = crate::db::logs::insert_log(
            db,
            &proxy_service::build_request_log(
                log_context,
                Some(status_code),
                Some(latency),
                input_tokens,
                output_tokens,
                None,
                false,
            ),
        )
        .await;

        Response::builder()
            .status(StatusCode::from_u16(status_code as u16).unwrap())
            .header("Content-Type", "application/json")
            .body(Body::from(resp_body))
            .unwrap()
    } else if is_openai {
        let converted =
            crate::adapter::openai::convert_error_response(&resp_bytes, status_code as u16);

        let _ = crate::db::logs::insert_log(
            db,
            &proxy_service::build_request_log(
                log_context,
                Some(status_code),
                Some(latency),
                0,
                0,
                converted
                    .get("error")
                    .and_then(|v| v.get("message"))
                    .and_then(|v| v.as_str())
                    .map(|v| v.to_string()),
                false,
            ),
        )
        .await;

        Response::builder()
            .status(StatusCode::from_u16(status_code as u16).unwrap())
            .header("Content-Type", "application/json")
            .body(Body::from(
                serde_json::to_vec(&converted).unwrap_or_default(),
            ))
            .unwrap()
    } else {
        Response::builder()
            .status(StatusCode::from_u16(status_code as u16).unwrap())
            .header("Content-Type", "application/json")
            .body(Body::from(resp_bytes.to_vec()))
            .unwrap()
    }
}

pub async fn handle_models(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let providers = crate::config::store::get_enabled_providers(&state.db)
        .await
        .unwrap_or_default();

    axum::Json(proxy_service::models_payload(&providers))
}

pub async fn handle_health() -> impl IntoResponse {
    axum::Json(serde_json::json!({ "status": "ok" }))
}

fn anthropic_error(status: u16, message: &str) -> Response {
    let body = serde_json::json!({
        "type": "error",
        "error": {
            "type": "invalid_request_error",
            "message": message
        }
    });
    Response::builder()
        .status(StatusCode::from_u16(status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR))
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_vec(&body).unwrap_or_default()))
        .unwrap()
}
