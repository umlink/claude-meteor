use serde_json::Value;

pub fn convert_error_response(openai_error: &[u8], status: u16) -> Value {
    let parsed = serde_json::from_slice::<Value>(openai_error).ok();
    let error = parsed.as_ref().and_then(|value| value.get("error"));

    let message = error
        .and_then(|value| value.get("message"))
        .and_then(|value| value.as_str())
        .unwrap_or("Upstream request failed");

    let error_type = error
        .and_then(|value| value.get("type"))
        .and_then(|value| value.as_str())
        .map(map_openai_error_type)
        .unwrap_or_else(|| default_anthropic_error_type(status));

    serde_json::json!({
        "type": "error",
        "error": {
            "type": error_type,
            "message": message
        }
    })
}

fn map_openai_error_type(error_type: &str) -> &str {
    match error_type {
        "invalid_request_error" => "invalid_request_error",
        "authentication_error" | "invalid_api_key" => "authentication_error",
        "permission_error" => "permission_error",
        "rate_limit_error" | "insufficient_quota" => "rate_limit_error",
        "server_error" => "api_error",
        _ => "api_error",
    }
}

fn default_anthropic_error_type(status: u16) -> &'static str {
    match status {
        400 => "invalid_request_error",
        401 => "authentication_error",
        403 => "permission_error",
        429 => "rate_limit_error",
        500..=599 => "api_error",
        _ => "api_error",
    }
}
