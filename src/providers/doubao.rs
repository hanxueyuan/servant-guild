//! Doubao LLM 2.0 Lite Provider Implementation
//!
//! This module implements the Provider trait for Doubao (ByteDance) LLM 2.0 Lite API.
//!
//! API Documentation: https://www.volcengine.com/docs/82379/1263482
//!
//! Features:
//! - Chat completions
//! - Native tool calling support
//! - Reasoning content support
//! - Token usage tracking

use crate::providers::traits::{
    ChatMessage, ChatResponse as ProviderChatResponse, Provider, ProviderCapabilities, TokenUsage,
    ToolCall as ProviderToolCall,
};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Doubao LLM 2.0 Lite provider
pub struct DoubaoProvider {
    /// API endpoint base URL
    base_url: String,
    /// API credential (API Key)
    credential: Option<String>,
    /// Maximum tokens override for responses
    max_tokens_override: Option<u32>,
    /// HTTP client with timeout
    client: Client,
}

impl DoubaoProvider {
    /// Create a new Doubao provider
    pub fn new(base_url: String, credential: Option<String>) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            base_url,
            credential,
            max_tokens_override: None,
            client,
        }
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub fn credential(&self) -> Option<&str> {
        self.credential.as_deref()
    }

    /// Set maximum tokens for responses
    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens_override = Some(max_tokens);
        self
    }

    /// Get the authorization header value
    fn get_auth_header(&self) -> Result<String, String> {
        self.credential
            .as_ref()
            .map(|c| format!("Bearer {}", c))
            .ok_or_else(|| "Doubao API credential not configured".to_string())
    }
}

/// Chat completion request for Doubao API
#[derive(Debug, Serialize)]
struct DoubaoChatRequest {
    /// Model identifier
    model: String,
    /// Messages in the conversation
    messages: Vec<DoubaoMessage>,
    /// Sampling temperature (0-1)
    temperature: f64,
    /// Maximum tokens in response
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    /// Tool definitions for function calling
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<DoubaoToolSpec>>,
    /// Tool choice strategy
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_choice: Option<String>,
}

/// Message in Doubao API format
#[derive(Debug, Serialize, Deserialize)]
struct DoubaoMessage {
    /// Message role (system, user, assistant, tool)
    role: String,
    /// Message content
    content: String,
    /// Tool call ID (for tool response messages)
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_call_id: Option<String>,
    /// Tool calls (for assistant messages)
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Vec<DoubaoToolCall>>,
    /// Reasoning content (for thinking models)
    #[serde(skip_serializing_if = "Option::is_none")]
    reasoning_content: Option<String>,
}

/// Tool specification for Doubao API
#[derive(Debug, Serialize, Deserialize)]
struct DoubaoToolSpec {
    #[serde(rename = "type")]
    kind: String,
    function: DoubaoToolFunctionSpec,
}

/// Tool function specification
#[derive(Debug, Serialize, Deserialize)]
struct DoubaoToolFunctionSpec {
    /// Tool name
    name: String,
    /// Tool description
    description: String,
    /// Tool parameters (JSON Schema)
    parameters: serde_json::Value,
}

/// Tool call in Doubao API format
#[derive(Debug, Serialize, Deserialize)]
struct DoubaoToolCall {
    /// Tool call ID
    id: String,
    /// Tool name
    #[serde(rename = "type")]
    kind: String,
    function: DoubaoToolFunctionCall,
}

/// Tool function call
#[derive(Debug, Serialize, Deserialize)]
struct DoubaoToolFunctionCall {
    /// Function name
    name: String,
    /// Function arguments (JSON string)
    arguments: String,
}

/// Chat completion response from Doubao API
#[derive(Debug, Deserialize)]
struct DoubaoChatResponse {
    /// Response choices
    choices: Vec<DoubaoChoice>,
    /// Token usage information
    usage: DoubaoUsage,
}

/// Choice in response
#[derive(Debug, Deserialize)]
struct DoubaoChoice {
    /// Message content
    message: DoubaoResponseMessage,
    /// Finish reason (stop, length, tool_calls)
    finish_reason: Option<String>,
}

/// Response message
#[derive(Debug, Deserialize)]
struct DoubaoResponseMessage {
    /// Text content
    #[serde(default)]
    content: Option<String>,
    /// Tool calls
    #[serde(default)]
    tool_calls: Option<Vec<DoubaoToolCall>>,
    /// Reasoning content (for thinking models)
    #[serde(default)]
    reasoning_content: Option<String>,
}

/// Token usage information
#[derive(Debug, Deserialize)]
struct DoubaoUsage {
    /// Input tokens
    prompt_tokens: u32,
    /// Output tokens
    completion_tokens: u32,
    /// Total tokens
    total_tokens: u32,
}

/// API error response
#[derive(Debug, Deserialize)]
struct DoubaoError {
    /// Error message
    message: String,
    /// Error type
    #[serde(rename = "type")]
    error_type: String,
}

impl DoubaoResponseMessage {
    /// Get effective content (content or reasoning_content as fallback)
    fn effective_content(&self) -> String {
        match &self.content {
            Some(c) if !c.is_empty() => c.clone(),
            _ => self.reasoning_content.clone().unwrap_or_default(),
        }
    }
}

#[async_trait]
impl Provider for DoubaoProvider {
    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            native_tool_calling: true,
            vision: false,
        }
    }

    async fn chat_with_system(
        &self,
        system_prompt: Option<&str>,
        message: &str,
        model: &str,
        temperature: f64,
    ) -> anyhow::Result<String> {
        let mut messages = Vec::new();

        // Add system prompt if provided
        if let Some(system) = system_prompt {
            messages.push(DoubaoMessage {
                role: "system".to_string(),
                content: system.to_string(),
                tool_call_id: None,
                tool_calls: None,
                reasoning_content: None,
            });
        }

        // Add user message
        messages.push(DoubaoMessage {
            role: "user".to_string(),
            content: message.to_string(),
            tool_call_id: None,
            tool_calls: None,
            reasoning_content: None,
        });

        let request = DoubaoChatRequest {
            model: model.to_string(),
            messages,
            temperature,
            max_tokens: self.max_tokens_override,
            tools: None,
            tool_choice: None,
        };

        let auth_header = self.get_auth_header().map_err(anyhow::Error::msg)?;
        let url = format!("{}/chat/completions", self.base_url);

        let response = self
            .client
            .post(&url)
            .header("Authorization", auth_header)
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to send request: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            anyhow::bail!("API request failed: {} - {}", status, text);
        }

        let resp: DoubaoChatResponse = response
            .json()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to parse response: {}", e))?;

        if resp.choices.is_empty() {
            anyhow::bail!("No choices in response");
        }

        Ok(resp.choices[0].message.effective_content())
    }

    async fn chat_with_history(
        &self,
        messages: &[ChatMessage],
        model: &str,
        temperature: f64,
    ) -> anyhow::Result<String> {
        let doubao_messages: Vec<DoubaoMessage> = messages
            .iter()
            .map(|m| DoubaoMessage {
                role: m.role.clone(),
                content: m.content.clone(),
                tool_call_id: None,
                tool_calls: None,
                reasoning_content: None,
            })
            .collect();

        let request = DoubaoChatRequest {
            model: model.to_string(),
            messages: doubao_messages,
            temperature,
            max_tokens: self.max_tokens_override,
            tools: None,
            tool_choice: None,
        };

        let auth_header = self.get_auth_header().map_err(anyhow::Error::msg)?;
        let url = format!("{}/chat/completions", self.base_url);

        let response = self
            .client
            .post(&url)
            .header("Authorization", auth_header)
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to send request: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            anyhow::bail!("API request failed: {} - {}", status, text);
        }

        let resp: DoubaoChatResponse = response
            .json()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to parse response: {}", e))?;

        if resp.choices.is_empty() {
            anyhow::bail!("No choices in response");
        }

        Ok(resp.choices[0].message.effective_content())
    }

    async fn chat_with_tools(
        &self,
        messages: &[ChatMessage],
        tools: &[serde_json::Value],
        model: &str,
        temperature: f64,
    ) -> anyhow::Result<ProviderChatResponse> {
        let tools: Option<Vec<DoubaoToolSpec>> = if tools.is_empty() {
            None
        } else {
            Some(
                tools
                    .iter()
                    .cloned()
                    .map(|tool| {
                        serde_json::from_value(tool)
                            .map_err(|e| anyhow::anyhow!("Invalid Doubao tool specification: {e}"))
                    })
                    .collect::<anyhow::Result<Vec<_>>>()?,
            )
        };

        // Convert messages to Doubao format
        let messages: Vec<DoubaoMessage> = messages
            .iter()
            .map(|m| DoubaoMessage {
                role: m.role.clone(),
                content: m.content.clone(),
                tool_call_id: None,
                tool_calls: None,
                reasoning_content: None,
            })
            .collect();

        let doubao_request = DoubaoChatRequest {
            model: model.to_string(),
            messages,
            temperature,
            max_tokens: self.max_tokens_override,
            tools,
            tool_choice: Some("auto".to_string()),
        };

        let auth_header = self.get_auth_header().map_err(anyhow::Error::msg)?;
        let url = format!("{}/chat/completions", self.base_url);

        let response = self
            .client
            .post(&url)
            .header("Authorization", auth_header)
            .header("Content-Type", "application/json")
            .json(&doubao_request)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to send request: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            anyhow::bail!("API request failed: {} - {}", status, text);
        }

        let resp: DoubaoChatResponse = response
            .json()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to parse response: {}", e))?;

        if resp.choices.is_empty() {
            anyhow::bail!("No choices in response");
        }

        let choice = &resp.choices[0];
        let message = &choice.message;

        // Convert tool calls
        let tool_calls: Vec<ProviderToolCall> = message
            .tool_calls
            .as_ref()
            .map(|calls| {
                calls
                    .iter()
                    .map(|tc| ProviderToolCall {
                        id: tc.id.clone(),
                        name: tc.function.name.clone(),
                        arguments: tc.function.arguments.clone(),
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok(ProviderChatResponse {
            text: Some(message.effective_content()),
            tool_calls,
            usage: Some(TokenUsage {
                input_tokens: Some(resp.usage.prompt_tokens as u64),
                output_tokens: Some(resp.usage.completion_tokens as u64),
            }),
            reasoning_content: message.reasoning_content.clone(),
        })
    }

    async fn warmup(&self) -> anyhow::Result<()> {
        // Warm up HTTP connection pool
        if self.credential.is_some() {
            let auth_header = self.get_auth_header().map_err(anyhow::Error::msg)?;
            let url = format!("{}/models", self.base_url);

            let _ = self
                .client
                .get(&url)
                .header("Authorization", auth_header)
                .timeout(Duration::from_secs(5))
                .send()
                .await;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::ToolSpec;

    #[test]
    fn test_doubao_provider_creation() {
        let provider = DoubaoProvider::new(
            "https://ark.cn-beijing.volces.com/api/v3".to_string(),
            Some("test-key".to_string()),
        );

        assert_eq!(
            provider.base_url,
            "https://ark.cn-beijing.volces.com/api/v3"
        );
        assert_eq!(provider.credential, Some("test-key".to_string()));
    }

    #[test]
    fn test_doubao_provider_with_max_tokens() {
        let provider = DoubaoProvider::new(
            "https://ark.cn-beijing.volces.com/api/v3".to_string(),
            Some("test-key".to_string()),
        )
        .with_max_tokens(4096);

        assert_eq!(provider.max_tokens_override, Some(4096));
    }

    #[test]
    fn test_doubao_capabilities() {
        let provider = DoubaoProvider::new(
            "https://ark.cn-beijing.volces.com/api/v3".to_string(),
            Some("test-key".to_string()),
        );

        let caps = provider.capabilities();
        assert!(caps.native_tool_calling);
        assert!(!caps.vision);
    }

    #[test]
    fn test_tool_spec_conversion() {
        let tool_spec = ToolSpec {
            name: "test_tool".to_string(),
            description: "A test tool".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "param1": {"type": "string"}
                }
            }),
        };

        let doubao_tool = DoubaoToolSpec {
            kind: "function".to_string(),
            function: DoubaoToolFunctionSpec {
                name: tool_spec.name.clone(),
                description: tool_spec.description.clone(),
                parameters: tool_spec.parameters.clone(),
            },
        };

        assert_eq!(doubao_tool.kind, "function");
        assert_eq!(doubao_tool.function.name, "test_tool");
    }
}
