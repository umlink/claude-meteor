pub(crate) struct AnthropicSseFrame {
    pub event: String,
    pub data: String,
}

impl std::fmt::Display for AnthropicSseFrame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "event: {}\ndata: {}\n\n", self.event, self.data)
    }
}

#[allow(dead_code)]
pub(crate) struct ToolCallState {
    pub index: u32,
    pub id: String,
    pub name: String,
}
