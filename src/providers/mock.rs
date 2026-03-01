use crate::providers::traits::{ChatMessage, ChatRequest, ChatResponse, Provider};
use async_trait::async_trait;

#[derive(Clone)]
pub struct MockProvider;

#[async_trait]
impl Provider for MockProvider {
    async fn chat_with_system(
        &self,
        _system_prompt: Option<&str>,
        _message: &str,
        _model: &str,
        _temperature: f64,
    ) -> anyhow::Result<String> {
        Ok("This is a mock response from the Wasm host stub.".to_string())
    }

    async fn chat(
        &self,
        _request: ChatRequest<'_>,
        _model: &str,
        _temperature: f64,
    ) -> anyhow::Result<ChatResponse> {
        Ok(ChatResponse {
            text: Some("Mock response".to_string()),
            tool_calls: vec![],
            usage: None,
            reasoning_content: None,
        })
    }
}
