use crate::config::provider::Provider;
use serde_json::{json, Value};

/// Convert Anthropic Messages API request to OpenAI Chat Completions request
pub fn convert_request(request: &Value, provider: &Provider) -> Result<Value, String> {
    let mut openai_request = json!({});

    let model = provider
        .model_mapping
        .as_deref()
        .map(str::trim)
        .filter(|mapping| !mapping.is_empty())
        .unwrap_or(
            request
                .get("model")
                .and_then(|m| m.as_str())
                .unwrap_or("gpt-4"),
        );
    openai_request["model"] = json!(model);

    if let Some(mt) = request.get("max_tokens") {
        openai_request["max_tokens"] = mt.clone();
    }

    let stream = request
        .get("stream")
        .and_then(|value| value.as_bool())
        .unwrap_or(true);
    openai_request["stream"] = json!(stream);
    if stream {
        openai_request["stream_options"] = json!({"include_usage": true});
    }

    let mut messages = Vec::new();

    if let Some(system) = request.get("system") {
        let system_content = match system {
            Value::String(s) => s.clone(),
            Value::Array(blocks) => blocks
                .iter()
                .filter_map(|b| b.get("text").and_then(|t| t.as_str()))
                .collect::<Vec<_>>()
                .join("\n"),
            _ => String::new(),
        };
        if !system_content.is_empty() {
            messages.push(json!({"role": "system", "content": system_content}));
        }
    }

    if let Some(msgs) = request.get("messages").and_then(|m| m.as_array()) {
        for msg in msgs {
            let role = msg.get("role").and_then(|r| r.as_str()).unwrap_or("user");
            let content = msg.get("content");

            if role == "assistant" {
                if let Some(Value::Array(blocks)) = content {
                    let mut text_parts = Vec::new();
                    let mut tool_calls = Vec::new();

                    for block in blocks {
                        match block.get("type").and_then(|t| t.as_str()) {
                            Some("text") => {
                                if let Some(text) = block.get("text").and_then(|t| t.as_str()) {
                                    text_parts.push(text.to_string());
                                }
                            }
                            Some("tool_use") => {
                                let id = block.get("id").and_then(|v| v.as_str()).unwrap_or("");
                                let name = block.get("name").and_then(|v| v.as_str()).unwrap_or("");
                                let default_input = json!({});
                                let input = block.get("input").unwrap_or(&default_input);
                                tool_calls.push(json!({
                                    "id": id,
                                    "type": "function",
                                    "function": {
                                        "name": name,
                                        "arguments": serde_json::to_string(input).unwrap_or_default()
                                    }
                                }));
                            }
                            _ => {}
                        }
                    }

                    let mut assistant_msg = json!({"role": "assistant"});
                    if !text_parts.is_empty() {
                        assistant_msg["content"] = json!(text_parts.join(""));
                    } else {
                        assistant_msg["content"] = Value::Null;
                    }
                    if !tool_calls.is_empty() {
                        assistant_msg["tool_calls"] = json!(tool_calls);
                    }
                    messages.push(assistant_msg);
                } else if let Some(Value::String(text)) = content {
                    messages.push(json!({"role": "assistant", "content": text}));
                }
            } else if role == "user" {
                if let Some(Value::Array(blocks)) = content {
                    let mut parts = Vec::new();
                    for block in blocks {
                        match block.get("type").and_then(|t| t.as_str()) {
                            Some("text") => {
                                if let Some(text) = block.get("text").and_then(|t| t.as_str()) {
                                    parts.push(json!({"type": "text", "text": text}));
                                }
                            }
                            Some("image") => {
                                if let Some(source) = block.get("source") {
                                    let media_type = source
                                        .get("media_type")
                                        .and_then(|m| m.as_str())
                                        .unwrap_or("image/png");
                                    let data =
                                        source.get("data").and_then(|d| d.as_str()).unwrap_or("");
                                    parts.push(json!({
                                        "type": "image_url",
                                        "image_url": {"url": format!("data:{};base64,{}", media_type, data)}
                                    }));
                                }
                            }
                            Some("tool_result") => {
                                let tool_use_id = block
                                    .get("tool_use_id")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("");
                                let result_content =
                                    block.get("content").and_then(|c| c.as_str()).unwrap_or("");
                                messages.push(json!({
                                    "role": "tool",
                                    "tool_call_id": tool_use_id,
                                    "content": result_content
                                }));
                            }
                            _ => {}
                        }
                    }
                    if !parts.is_empty() {
                        messages.push(json!({"role": "user", "content": parts}));
                    }
                } else if let Some(Value::String(text)) = content {
                    messages.push(json!({"role": "user", "content": text}));
                }
            }
        }
    }

    openai_request["messages"] = json!(messages);

    if let Some(tools) = request.get("tools").and_then(|t| t.as_array()) {
        let openai_tools: Vec<Value> = tools
            .iter()
            .map(|tool| {
                json!({
                    "type": "function",
                    "function": {
                        "name": tool["name"],
                        "description": tool.get("description").unwrap_or(&json!("")),
                        "parameters": tool.get("input_schema").unwrap_or(&json!({}))
                    }
                })
            })
            .collect();
        openai_request["tools"] = json!(openai_tools);
    }

    if let Some(tc) = request.get("tool_choice") {
        let openai_tc = match tc {
            Value::String(s) if s == "auto" => json!({"type": "auto"}),
            Value::String(s) if s == "any" => json!({"type": "required"}),
            Value::String(s) if s == "none" => json!({"type": "none"}),
            Value::Object(obj) => {
                if obj.get("type").and_then(|t| t.as_str()) == Some("tool") {
                    json!({
                        "type": "function",
                        "function": {"name": obj.get("name").and_then(|n| n.as_str()).unwrap_or("")}
                    })
                } else {
                    tc.clone()
                }
            }
            _ => json!({"type": "auto"}),
        };
        openai_request["tool_choice"] = openai_tc;
    }

    if let Some(ss) = request.get("stop_sequences") {
        openai_request["stop"] = ss.clone();
    }

    Ok(openai_request)
}
