pub mod anthropic;
pub mod openai;

use bytes::Bytes;
use futures::Stream;
use std::pin::Pin;
use tokio::sync::oneshot;

#[derive(Debug, Clone)]
pub struct StreamSummary {
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub error_message: Option<String>,
}

pub struct MonitoredSseStream {
    pub stream: Pin<Box<dyn Stream<Item = Result<Bytes, std::io::Error>> + Send>>,
    pub summary: oneshot::Receiver<StreamSummary>,
}
