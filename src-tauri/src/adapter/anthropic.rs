use crate::adapter::{MonitoredSseStream, StreamSummary};
use bytes::Bytes;
use futures::stream::StreamExt;
use futures::Stream;
use serde_json::Value;
use tokio::sync::{mpsc, oneshot};

pub fn monitor_sse_stream(
    stream: impl Stream<Item = Result<Bytes, reqwest::Error>> + Send + 'static,
) -> MonitoredSseStream {
    let (tx, rx) = mpsc::channel::<Result<Bytes, std::io::Error>>(32);
    let (summary_tx, summary_rx) = oneshot::channel::<StreamSummary>();

    tokio::spawn(async move {
        let mut stream = Box::pin(stream);
        let mut buffer = String::new();
        let mut input_tokens = 0_i64;
        let mut output_tokens = 0_i64;
        let mut error_message = None;

        while let Some(chunk_result) = stream.next().await {
            match chunk_result {
                Ok(chunk) => {
                    let _ = tx.send(Ok(chunk.clone())).await;
                    buffer.push_str(&String::from_utf8_lossy(&chunk));

                    while let Some(pos) = buffer.find("\n\n") {
                        let event_text = buffer[..pos].to_string();
                        buffer = buffer[pos + 2..].to_string();

                        for line in event_text.lines() {
                            if let Some(data) = line.strip_prefix("data: ") {
                                let parsed: Value = match serde_json::from_str(data) {
                                    Ok(v) => v,
                                    Err(_) => continue,
                                };

                                if let Some(message_usage) = parsed
                                    .get("message")
                                    .and_then(|message| message.get("usage"))
                                {
                                    if let Some(tokens) = message_usage
                                        .get("input_tokens")
                                        .and_then(|value| value.as_i64())
                                    {
                                        input_tokens = tokens;
                                    }
                                    if let Some(tokens) = message_usage
                                        .get("output_tokens")
                                        .and_then(|value| value.as_i64())
                                    {
                                        output_tokens = tokens;
                                    }
                                }

                                if let Some(usage) = parsed.get("usage") {
                                    if let Some(tokens) =
                                        usage.get("input_tokens").and_then(|value| value.as_i64())
                                    {
                                        input_tokens = tokens;
                                    }
                                    if let Some(tokens) =
                                        usage.get("output_tokens").and_then(|value| value.as_i64())
                                    {
                                        output_tokens = tokens;
                                    }
                                }
                            }
                        }
                    }
                }
                Err(error) => {
                    error_message = Some(error.to_string());
                    let io_error = std::io::Error::other(error.to_string());
                    let _ = tx.send(Err(io_error)).await;
                    break;
                }
            }
        }

        let _ = summary_tx.send(StreamSummary {
            input_tokens,
            output_tokens,
            error_message,
        });
    });

    MonitoredSseStream {
        stream: Box::pin(tokio_stream::wrappers::ReceiverStream::new(rx)),
        summary: summary_rx,
    }
}
