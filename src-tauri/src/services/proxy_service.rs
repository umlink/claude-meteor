use crate::config::provider::{AuthHeader, Protocol, Provider};
use crate::config::store;
use crate::db::logs::{DbConn, RequestLog};
use crate::proxy::router::match_provider;
use serde_json::Value;
use std::time::Instant;

#[derive(Clone)]
pub struct ResolvedProxyRequest {
    pub model: String,
    pub provider: Provider,
    pub upstream_url: String,
    pub is_openai: bool,
    pub upstream_body: Vec<u8>,
    pub is_streaming: bool,
}

#[derive(Clone)]
pub struct RequestLogContext {
    pub request_id: String,
    pub model: String,
    pub provider_id: String,
    pub provider_name: String,
    pub protocol: String,
    pub upstream_url: String,
}

pub async fn resolve_request(
    db: &DbConn,
    request_json: &mut Value,
    raw_body: &[u8],
) -> Result<ResolvedProxyRequest, String> {
    let model = request_json
        .get("model")
        .and_then(|m| m.as_str())
        .unwrap_or("")
        .to_string();

    let providers = store::get_enabled_providers(db).await?;
    let provider = match_provider(&model, &providers)
        .cloned()
        .ok_or("No enabled provider configured".to_string())?;

    let upstream_model = provider
        .model_mapping
        .as_deref()
        .map(str::trim)
        .filter(|m| !m.is_empty())
        .unwrap_or(&model)
        .to_string();

    let is_openai = provider.protocol == Protocol::OpenAI;
    let upstream_path = if is_openai {
        "/v1/chat/completions"
    } else {
        "/v1/messages"
    };
    let upstream_url = format!(
        "{}{}",
        provider.base_url.trim_end_matches('/'),
        upstream_path
    );

    if let Some(obj) = request_json.as_object_mut() {
        obj.insert("model".to_string(), Value::String(upstream_model.clone()));
    }

    tracing::info!(
        "Routing model {} -> provider {} [label={}] upstream model {}",
        model,
        provider.name,
        provider.keyword,
        upstream_model
    );

    let upstream_body = if is_openai {
        let converted = crate::adapter::openai::convert_request(request_json, &provider)?;
        serde_json::to_vec(&converted).unwrap_or_else(|_| raw_body.to_vec())
    } else {
        serde_json::to_vec(request_json).map_err(|e| e.to_string())?
    };

    let is_streaming = request_json
        .get("stream")
        .and_then(|s| s.as_bool())
        .unwrap_or(true);

    Ok(ResolvedProxyRequest {
        model,
        provider,
        upstream_url,
        is_openai,
        upstream_body,
        is_streaming,
    })
}

pub fn apply_auth_headers(
    req_builder: reqwest::RequestBuilder,
    provider: &Provider,
    api_key: &str,
    is_openai: bool,
) -> reqwest::RequestBuilder {
    match provider.auth_header {
        AuthHeader::ApiKey => {
            let req_builder = req_builder.header("x-api-key", api_key);
            if is_openai {
                req_builder.header("authorization", format!("Bearer {}", api_key))
            } else {
                req_builder
            }
        }
        AuthHeader::Bearer => req_builder.header("authorization", format!("Bearer {}", api_key)),
    }
}

pub fn make_log_context(request_id: String, resolved: &ResolvedProxyRequest) -> RequestLogContext {
    RequestLogContext {
        request_id,
        model: resolved.model.clone(),
        provider_id: resolved.provider.id.clone(),
        provider_name: resolved.provider.name.clone(),
        protocol: resolved.provider.protocol.as_str().to_string(),
        upstream_url: resolved.upstream_url.clone(),
    }
}

pub fn build_request_log(
    context: &RequestLogContext,
    status_code: Option<i32>,
    latency_ms: Option<i64>,
    input_tokens: i64,
    output_tokens: i64,
    error_message: Option<String>,
    is_streaming: bool,
) -> RequestLog {
    RequestLog {
        id: 0,
        request_id: context.request_id.clone(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        model: context.model.clone(),
        provider_id: context.provider_id.clone(),
        provider_name: context.provider_name.clone(),
        protocol: context.protocol.clone(),
        upstream_url: context.upstream_url.clone(),
        status_code,
        latency_ms,
        input_tokens,
        output_tokens,
        error_message,
        is_streaming,
    }
}

pub async fn log_upstream_send_error(
    db: &DbConn,
    context: &RequestLogContext,
    start: Instant,
    error: &reqwest::Error,
) {
    let latency = start.elapsed().as_millis() as i64;
    let _ = crate::db::logs::insert_log(
        db,
        &build_request_log(
            context,
            None,
            Some(latency),
            0,
            0,
            Some(error.to_string()),
            true,
        ),
    )
    .await;
}

pub fn spawn_stream_logger(
    db: &DbConn,
    summary: tokio::sync::oneshot::Receiver<crate::adapter::StreamSummary>,
    context: RequestLogContext,
    status_code: i32,
    start: Instant,
) {
    let db = db.clone();

    tokio::spawn(async move {
        let summary = summary.await.unwrap_or(crate::adapter::StreamSummary {
            input_tokens: 0,
            output_tokens: 0,
            error_message: Some("Streaming summary unavailable".to_string()),
        });
        let latency = start.elapsed().as_millis() as i64;
        let _ = crate::db::logs::insert_log(
            &db,
            &build_request_log(
                &context,
                Some(status_code),
                Some(latency),
                summary.input_tokens,
                summary.output_tokens,
                summary.error_message,
                true,
            ),
        )
        .await;
    });
}

pub fn models_payload(providers: &[Provider]) -> serde_json::Value {
    let models: Vec<Value> = providers
        .iter()
        .map(|provider| {
            serde_json::json!({
                "id": format!("claude-{}-4-6", provider.keyword),
                "display_name": provider.name,
            })
        })
        .collect();

    serde_json::json!({
        "data": models,
        "object": "list"
    })
}
