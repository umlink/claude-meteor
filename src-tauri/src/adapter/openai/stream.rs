use crate::adapter::openai::types::{AnthropicSseFrame, ToolCallState};
use crate::adapter::{MonitoredSseStream, StreamSummary};
use bytes::Bytes;
use futures::stream::StreamExt;
use serde_json::{json, Value};
use tokio::sync::{mpsc, oneshot};

/// Convert OpenAI SSE stream to Anthropic SSE stream
pub fn convert_sse_stream(
    stream: impl futures::Stream<Item = Result<Bytes, reqwest::Error>> + Send + 'static,
    model: &str,
    model_mapping: Option<&str>,
) -> MonitoredSseStream {
    let response_model = model_mapping.unwrap_or(model).to_string();
    let msg_id = format!(
        "msg-{}",
        uuid::Uuid::new_v4().to_string().replace("-", "")[..24].to_string()
    );

    let (tx, rx) = mpsc::channel::<Result<Bytes, std::io::Error>>(32);
    let (summary_tx, summary_rx) = oneshot::channel::<StreamSummary>();

    tokio::spawn(async move {
        let mut buffer = String::new();
        let mut message_started = false;
        let mut message_finished = false;
        let mut text_block_index: Option<u32> = None;
        let mut content_block_index: u32 = 0;
        let mut active_tool_calls: std::collections::HashMap<usize, ToolCallState> =
            std::collections::HashMap::new();
        let mut input_tokens: u64 = 0;
        let mut output_tokens: u64 = 0;
        let mut error_message = None;

        let mut stream = Box::pin(stream);

        while let Some(chunk_result) = stream.next().await {
            let chunk = match chunk_result {
                Ok(c) => c,
                Err(e) => {
                    error_message = Some(e.to_string());
                    let _ = tx.send(Err(std::io::Error::other(e.to_string()))).await;
                    break;
                }
            };

            buffer.push_str(&String::from_utf8_lossy(&chunk));

            while let Some(pos) = buffer.find("\n\n") {
                let event_text = buffer[..pos].to_string();
                buffer = buffer[pos + 2..].to_string();

                for line in event_text.lines() {
                    if let Some(data) = line.strip_prefix("data: ") {
                        if data == "[DONE]" {
                            if !message_finished {
                                close_all_blocks(&tx, &active_tool_calls, text_block_index.take())
                                    .await;
                                active_tool_calls.clear();

                                send_message_end(&tx, output_tokens).await;
                                message_finished = true;
                            }
                            continue;
                        }

                        let parsed: Value = match serde_json::from_str(data) {
                            Ok(v) => v,
                            Err(_) => continue,
                        };

                        extract_usage(&parsed, &mut input_tokens, &mut output_tokens);

                        let choice = parsed.get("choices").and_then(|c| c.get(0));
                        if choice.is_none() {
                            continue;
                        }
                        let choice = choice.unwrap();
                        let default_delta = json!({});
                        let delta = choice.get("delta").unwrap_or(&default_delta);
                        let finish_reason = choice.get("finish_reason").and_then(|f| f.as_str());

                        if !message_started {
                            send_message_start(&tx, &msg_id, &response_model, input_tokens).await;
                            message_started = true;
                        }

                        handle_text_content(
                            &tx,
                            delta,
                            &mut text_block_index,
                            &mut content_block_index,
                        )
                        .await;

                        handle_tool_calls(
                            &tx,
                            delta,
                            &mut active_tool_calls,
                            &mut text_block_index,
                            &mut content_block_index,
                        )
                        .await;

                        if let Some(reason) = finish_reason {
                            if !message_finished {
                                close_all_blocks(&tx, &active_tool_calls, text_block_index.take())
                                    .await;
                                active_tool_calls.clear();

                                send_finish(&tx, reason, output_tokens).await;
                                message_finished = true;
                            }
                        }
                    }
                }
            }
        }

        // If stream ended without [DONE], force close
        if message_started && !message_finished {
            close_all_blocks(&tx, &active_tool_calls, text_block_index.take()).await;
            send_message_end(&tx, output_tokens).await;
        }

        let _ = summary_tx.send(StreamSummary {
            input_tokens: input_tokens as i64,
            output_tokens: output_tokens as i64,
            error_message,
        });
    });

    MonitoredSseStream {
        stream: Box::pin(tokio_stream::wrappers::ReceiverStream::new(rx).map(|item| item)),
        summary: summary_rx,
    }
}

fn extract_usage(parsed: &Value, input_tokens: &mut u64, output_tokens: &mut u64) {
    if let Some(usage) = parsed.get("usage") {
        if let Some(pt) = usage.get("prompt_tokens").and_then(|v| v.as_u64()) {
            *input_tokens = pt;
        }
        if let Some(ct) = usage.get("completion_tokens").and_then(|v| v.as_u64()) {
            *output_tokens = ct;
        }
    }
}

async fn send_message_start(
    tx: &mpsc::Sender<Result<Bytes, std::io::Error>>,
    msg_id: &str,
    response_model: &str,
    input_tokens: u64,
) {
    let frame = AnthropicSseFrame {
        event: "message_start".to_string(),
        data: json!({
            "type": "message_start",
            "message": {
                "id": msg_id,
                "type": "message",
                "role": "assistant",
                "content": [],
                "model": response_model,
                "usage": {"input_tokens": input_tokens, "output_tokens": 0}
            }
        })
        .to_string(),
    };
    let _ = tx.send(Ok(Bytes::from(frame.to_string()))).await;
}

async fn handle_text_content(
    tx: &mpsc::Sender<Result<Bytes, std::io::Error>>,
    delta: &Value,
    text_block_index: &mut Option<u32>,
    content_block_index: &mut u32,
) {
    if let Some(content) = delta.get("content").and_then(|c| c.as_str()) {
        if !content.is_empty() {
            if text_block_index.is_none() {
                let block_index = *content_block_index;
                let start_frame = AnthropicSseFrame {
                    event: "content_block_start".to_string(),
                    data: json!({
                        "type": "content_block_start",
                        "index": block_index,
                        "content_block": {"type": "text", "text": ""}
                    })
                    .to_string(),
                };
                let _ = tx.send(Ok(Bytes::from(start_frame.to_string()))).await;
                *text_block_index = Some(block_index);
            }

            let block_index = text_block_index.unwrap_or(*content_block_index);
            let delta_frame = AnthropicSseFrame {
                event: "content_block_delta".to_string(),
                data: json!({
                    "type": "content_block_delta",
                    "index": block_index,
                    "delta": {"type": "text_delta", "text": content}
                })
                .to_string(),
            };
            let _ = tx.send(Ok(Bytes::from(delta_frame.to_string()))).await;
        }
    }
}

async fn handle_tool_calls(
    tx: &mpsc::Sender<Result<Bytes, std::io::Error>>,
    delta: &Value,
    active_tool_calls: &mut std::collections::HashMap<usize, ToolCallState>,
    text_block_index: &mut Option<u32>,
    content_block_index: &mut u32,
) {
    let tool_calls = match delta.get("tool_calls").and_then(|t| t.as_array()) {
        Some(tc) => tc,
        None => return,
    };

    for tc in tool_calls {
        let tc_index = tc.get("index").and_then(|i| i.as_u64()).unwrap_or(0) as usize;
        let tc_id = tc.get("id").and_then(|i| i.as_str()).unwrap_or("");
        let fn_name = tc
            .get("function")
            .and_then(|f| f.get("name"))
            .and_then(|n| n.as_str())
            .unwrap_or("");
        let fn_args = tc
            .get("function")
            .and_then(|f| f.get("arguments"))
            .and_then(|a| a.as_str())
            .unwrap_or("");

        if !tc_id.is_empty() {
            // Close text block if open
            if let Some(index) = text_block_index.take() {
                let close_frame = AnthropicSseFrame {
                    event: "content_block_stop".to_string(),
                    data: json!({"type": "content_block_stop", "index": index}).to_string(),
                };
                let _ = tx.send(Ok(Bytes::from(close_frame.to_string()))).await;
                *content_block_index += 1;
            }

            let start_frame = AnthropicSseFrame {
                event: "content_block_start".to_string(),
                data: json!({
                    "type": "content_block_start",
                    "index": *content_block_index,
                    "content_block": {
                        "type": "tool_use",
                        "id": tc_id,
                        "name": fn_name,
                        "input": {}
                    }
                })
                .to_string(),
            };
            let _ = tx.send(Ok(Bytes::from(start_frame.to_string()))).await;

            active_tool_calls.insert(
                tc_index,
                ToolCallState {
                    index: *content_block_index,
                    id: tc_id.to_string(),
                    name: fn_name.to_string(),
                },
            );

            *content_block_index += 1;
        }

        if !fn_args.is_empty() {
            let tc_state = active_tool_calls.get(&tc_index);
            let block_index = tc_state
                .map(|s| s.index)
                .unwrap_or(*content_block_index - 1);
            let delta_frame = AnthropicSseFrame {
                event: "content_block_delta".to_string(),
                data: json!({
                    "type": "content_block_delta",
                    "index": block_index,
                    "delta": {"type": "input_json_delta", "partial_json": fn_args}
                })
                .to_string(),
            };
            let _ = tx.send(Ok(Bytes::from(delta_frame.to_string()))).await;
        }
    }
}

async fn close_all_blocks(
    tx: &mpsc::Sender<Result<Bytes, std::io::Error>>,
    active_tool_calls: &std::collections::HashMap<usize, ToolCallState>,
    text_block_index: Option<u32>,
) {
    for tc_state in active_tool_calls.values() {
        let close_frame = AnthropicSseFrame {
            event: "content_block_stop".to_string(),
            data: json!({"type": "content_block_stop", "index": tc_state.index}).to_string(),
        };
        let _ = tx.send(Ok(Bytes::from(close_frame.to_string()))).await;
    }

    if let Some(index) = text_block_index {
        let frame = AnthropicSseFrame {
            event: "content_block_stop".to_string(),
            data: json!({"type": "content_block_stop", "index": index}).to_string(),
        };
        let _ = tx.send(Ok(Bytes::from(frame.to_string()))).await;
    }
}

async fn send_finish(
    tx: &mpsc::Sender<Result<Bytes, std::io::Error>>,
    reason: &str,
    output_tokens: u64,
) {
    let stop_reason = match reason {
        "stop" => "end_turn",
        "tool_calls" => "tool_use",
        "length" => "max_tokens",
        _ => "end_turn",
    };

    let delta_frame = AnthropicSseFrame {
        event: "message_delta".to_string(),
        data: json!({
            "type": "message_delta",
            "delta": {"stop_reason": stop_reason},
            "usage": {"output_tokens": output_tokens}
        })
        .to_string(),
    };
    let _ = tx.send(Ok(Bytes::from(delta_frame.to_string()))).await;

    let stop_frame = AnthropicSseFrame {
        event: "message_stop".to_string(),
        data: json!({"type": "message_stop"}).to_string(),
    };
    let _ = tx.send(Ok(Bytes::from(stop_frame.to_string()))).await;
}

async fn send_message_end(tx: &mpsc::Sender<Result<Bytes, std::io::Error>>, output_tokens: u64) {
    let delta_frame = AnthropicSseFrame {
        event: "message_delta".to_string(),
        data: json!({
            "type": "message_delta",
            "delta": {"stop_reason": "end_turn"},
            "usage": {"output_tokens": output_tokens}
        })
        .to_string(),
    };
    let _ = tx.send(Ok(Bytes::from(delta_frame.to_string()))).await;

    let stop_frame = AnthropicSseFrame {
        event: "message_stop".to_string(),
        data: json!({"type": "message_stop"}).to_string(),
    };
    let _ = tx.send(Ok(Bytes::from(stop_frame.to_string()))).await;
}
