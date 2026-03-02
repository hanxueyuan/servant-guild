use crate::runtime::state::HostState;
use crate::runtime::bindings::zeroclaw::host::llm::{
    Host, CompletionRequest, CompletionResponse, EmbeddingRequest,
    ToolDefinition, ToolCall as WitToolCall, UsageStats, Message, Role,
};
use crate::providers::traits::{ChatRequest, ChatMessage};
use crate::tools::traits::ToolSpec;
use anyhow::Result;

#[async_trait::async_trait]
impl Host for HostState {
    async fn chat(&mut self, req: CompletionRequest) -> Result<CompletionResponse, String> {
        let messages: Vec<ChatMessage> = req.messages.into_iter().map(|m| {
            match m.role {
                Role::System => ChatMessage::system(m.content),
                Role::User => ChatMessage::user(m.content),
                Role::Assistant => ChatMessage::assistant(m.content),
                Role::Tool => ChatMessage::tool(m.content),
            }
        }).collect();

        let tools: Vec<ToolSpec> = req.tools.into_iter().map(|t| {
            ToolSpec {
                name: t.name,
                description: t.description,
                parameters: serde_json::from_str(&t.parameters).unwrap_or(serde_json::Value::Null),
            }
        }).collect();

        let request = ChatRequest {
            messages: &messages,
            tools: if tools.is_empty() { None } else { Some(&tools) },
        };

        let response = self.provider.chat(
            request,
            &req.model,
            req.temperature.map(|t| t as f64).unwrap_or(0.7)
        ).await.map_err(|e| e.to_string())?;

        Ok(CompletionResponse {
            content: response.text,
            tool_calls: response.tool_calls.into_iter().map(|tc| WitToolCall {
                id: tc.id,
                name: tc.name,
                arguments: tc.arguments,
            }).collect(),
            usage: UsageStats {
                prompt_tokens: response.usage.as_ref().and_then(|u| u.input_tokens).unwrap_or(0) as u32,
                completion_tokens: response.usage.as_ref().and_then(|u| u.output_tokens).unwrap_or(0) as u32,
                total_tokens: 0, // Calculated by Guest if needed
            },
        })
    }

    async fn embed(&mut self, req: EmbeddingRequest) -> Result<Vec<f32>, String> {
        let mem = self
            .memory
            .as_ref()
            .ok_or_else(|| "memory backend is not configured".to_string())?;
        mem.embed_one(&req.input).await.map_err(|e| e.to_string())
    }
}
