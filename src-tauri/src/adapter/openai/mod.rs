mod error;
mod request;
mod response;
mod stream;
mod types;

pub use error::convert_error_response;
pub use request::convert_request;
pub use response::convert_response;
pub use stream::convert_sse_stream;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::provider::{AuthHeader, Protocol, Provider};
    use bytes::Bytes;
    use futures::stream;
    use futures::StreamExt;
    use serde_json::json;

    fn test_provider() -> Provider {
        Provider {
            id: "provider-1".to_string(),
            name: "OpenAI".to_string(),
            base_url: "https://api.openai.com".to_string(),
            api_key_enc: "secret".to_string(),
            protocol: Protocol::OpenAI,
            model_mapping: Some("gpt-4o-mini".to_string()),
            auth_header: AuthHeader::Bearer,
            keyword: "opus".to_string(),
            enabled: true,
            sort_order: 0,
        }
    }

    #[test]
    fn convert_request_preserves_stream_setting() {
        let provider = test_provider();
        let request = json!({
            "model": "claude-opus-4-6",
            "stream": false,
            "messages": [{"role": "user", "content": "hello"}]
        });

        let converted = convert_request(&request, &provider).expect("request should convert");

        assert_eq!(
            converted.get("stream"),
            Some(&serde_json::Value::Bool(false))
        );
        assert!(converted.get("stream_options").is_none());
    }

    #[tokio::test]
    async fn convert_sse_stream_finishes_once() {
        let openai_events = vec![
            Ok(Bytes::from(
                "data: {\"choices\":[{\"delta\":{\"content\":\"Hello\"},\"index\":0}]}\n\n",
            )),
            Ok(Bytes::from(
                "data: {\"choices\":[{\"delta\":{},\"finish_reason\":\"stop\",\"index\":0}],\"usage\":{\"prompt_tokens\":10,\"completion_tokens\":5}}\n\n",
            )),
            Ok(Bytes::from("data: [DONE]\n\n")),
        ];

        let monitored = convert_sse_stream(stream::iter(openai_events), "claude-opus-4-6", None);
        let output = monitored
            .stream
            .collect::<Vec<_>>()
            .await
            .into_iter()
            .map(|item| item.expect("stream item should succeed"))
            .map(|bytes| String::from_utf8(bytes.to_vec()).expect("valid utf8"))
            .collect::<String>();

        let summary = monitored.summary.await.expect("summary should resolve");

        assert_eq!(output.matches("event: message_stop").count(), 1);
        assert_eq!(output.matches("event: message_delta").count(), 1);
        assert_eq!(summary.input_tokens, 10);
        assert_eq!(summary.output_tokens, 5);
    }

    #[test]
    fn convert_error_response_maps_openai_shape() {
        let openai_error = br#"{
            "error": {
                "message": "Rate limit exceeded",
                "type": "rate_limit_error"
            }
        }"#;

        let converted = convert_error_response(openai_error, 429);

        assert_eq!(converted["type"], "error");
        assert_eq!(converted["error"]["type"], "rate_limit_error");
        assert_eq!(converted["error"]["message"], "Rate limit exceeded");
    }
}
