// Copyright 2025 ZeroClaw Labs.
// Modified by LiteFlow-Rust contributors.
// Source: https://github.com/zeroclaw-labs/zeroclaw (Apache-2.0)
// SPDX-License-Identifier: Apache-2.0
//
// 本文件衍生自 ZeroClaw 项目 src/providers/compatible.rs。
// 修改：
// - import 路径调整为 liteflow-agent-provider-core；
// - 精简：去掉 websocket 流式、responses API fallback、multimodal 图片处理，
//   保留标准 OpenAI /v1/chat/completions + native tool calling + 可配置 base_url/auth_style。
// "ZeroClaw" 是 ZeroClaw Labs 的商标；本项目与其无官方关联。

//! 通用 OpenAI 兼容 provider。
//!
//! 适用于所有遵循 `/v1/chat/completions` 格式的服务：
//! DeepSeek、Kimi（Moonshot）、Minimax、Qwen（通义）、Groq、Mistral、xAI、Venice 等。

use super::AuthStyle;
use async_trait::async_trait;
use liteflow_agent_provider_core::util::api_error;
use liteflow_agent_provider_core::{
    ChatMessage, ChatRequest as ProviderChatRequest, ChatResponse as ProviderChatResponse,
    Provider, ProviderCapabilities, TokenUsage, ToolCall as ProviderToolCall, ToolSpec,
};
use reqwest::Client;
use serde::{Deserialize, Serialize};

/// 调用 OpenAI Chat Completions 协议的通用兼容提供商。
///
/// 对应 Java: 无（Rust 提供商基础设施；源自 ZeroClaw `CompatibleProvider`）。
pub struct CompatibleProvider {
    name: String,
    base_url: String,
    credential: Option<String>,
    auth_style: AuthStyle,
    native_tool_calling: bool,
    max_tokens_override: Option<u32>,
    client: Client,
}

impl CompatibleProvider {
    /// 创建通用兼容 provider。
    ///
    /// - `name`：provider 名称（用于错误信息，如 "deepseek"）
    /// - `base_url`：API 基址（如 `https://api.deepseek.com/v1`）
    /// - `credential`：API key
    #[must_use]
    pub fn new(
        name: impl Into<String>,
        base_url: impl Into<String>,
        credential: Option<&str>,
    ) -> Self {
        Self {
            name: name.into(),
            base_url: base_url.into(),
            credential: credential.map(ToString::to_string),
            auth_style: AuthStyle::default(),
            native_tool_calling: true,
            max_tokens_override: None,
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(120))
                .connect_timeout(std::time::Duration::from_secs(10))
                .build()
                .unwrap_or_else(|_| Client::new()),
        }
    }

    /// 设置 auth style。
    #[must_use]
    pub fn with_auth_style(mut self, style: AuthStyle) -> Self {
        self.auth_style = style;
        self
    }

    /// 设置是否支持 native tool calling。
    #[must_use]
    pub fn with_native_tool_calling(mut self, enabled: bool) -> Self {
        self.native_tool_calling = enabled;
        self
    }

    /// 设置 max_tokens 覆盖。
    #[must_use]
    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        if max_tokens > 0 {
            self.max_tokens_override = Some(max_tokens);
        }
        self
    }

    fn chat_url(&self) -> String {
        let base = self.base_url.trim_end_matches('/');
        format!("{base}/chat/completions")
    }

    fn add_auth_header(&self, req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        match (&self.auth_style, &self.credential) {
            (AuthStyle::Bearer, Some(key)) => req.header("Authorization", format!("Bearer {key}")),
            (AuthStyle::Custom, Some(key)) => req.header("api-key", key),
            (_, None) => req,
        }
    }

    fn convert_tools(tools: Option<&[ToolSpec]>) -> Option<Vec<NativeToolSpec>> {
        let items = tools?;
        if items.is_empty() {
            return None;
        }
        Some(
            items
                .iter()
                .map(|tool| NativeToolSpec {
                    kind: "function".to_string(),
                    function: NativeToolFunctionSpec {
                        name: tool.name.clone(),
                        description: tool.description.clone(),
                        parameters: tool.parameters.clone(),
                    },
                })
                .collect(),
        )
    }

    fn convert_messages(messages: &[ChatMessage]) -> Vec<NativeMessage> {
        messages
            .iter()
            .map(|m| {
                if m.role == "assistant" {
                    if let Ok(value) = serde_json::from_str::<serde_json::Value>(&m.content) {
                        if let Some(tool_calls_value) = value.get("tool_calls") {
                            if let Ok(parsed_calls) =
                                serde_json::from_value::<Vec<ProviderToolCall>>(tool_calls_value.clone())
                            {
                                let tool_calls = parsed_calls
                                    .into_iter()
                                    .map(|tc| NativeToolCall {
                                        id: Some(tc.id),
                                        kind: Some("function".to_string()),
                                        function: NativeFunctionCall {
                                            name: tc.name,
                                            arguments: tc.arguments,
                                        },
                                    })
                                    .collect::<Vec<_>>();
                                let content = value
                                    .get("content")
                                    .and_then(serde_json::Value::as_str)
                                    .map(ToString::to_string);
                                let reasoning_content = value
                                    .get("reasoning_content")
                                    .and_then(serde_json::Value::as_str)
                                    .map(ToString::to_string);
                                return NativeMessage {
                                    role: "assistant".to_string(),
                                    content,
                                    tool_call_id: None,
                                    tool_calls: Some(tool_calls),
                                    reasoning_content,
                                };
                            }
                        }
                    }
                }

                if m.role == "tool" {
                    if let Ok(value) = serde_json::from_str::<serde_json::Value>(&m.content) {
                        let tool_call_id = value
                            .get("tool_call_id")
                            .and_then(serde_json::Value::as_str)
                            .map(ToString::to_string);
                        let content = value
                            .get("content")
                            .and_then(serde_json::Value::as_str)
                            .map(ToString::to_string)
                            .unwrap_or_else(|| m.content.clone());
                        return NativeMessage {
                            role: "tool".to_string(),
                            content: Some(content),
                            tool_call_id,
                            tool_calls: None,
                            reasoning_content: None,
                        };
                    }
                }

                NativeMessage {
                    role: m.role.clone(),
                    content: Some(m.content.clone()),
                    tool_call_id: None,
                    tool_calls: None,
                    reasoning_content: None,
                }
            })
            .collect()
    }
}

// ── Request/Response types ──

#[derive(Debug, Serialize)]
struct SimpleChatRequest {
    model: String,
    messages: Vec<SimpleMessage>,
    temperature: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
}

#[derive(Debug, Serialize)]
struct SimpleMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct SimpleChatResponse {
    choices: Vec<SimpleChoice>,
}

#[derive(Debug, Deserialize)]
struct SimpleChoice {
    message: SimpleResponseMessage,
}

#[derive(Debug, Deserialize)]
struct SimpleResponseMessage {
    content: String,
}

#[derive(Debug, Serialize)]
struct NativeChatRequest {
    model: String,
    messages: Vec<NativeMessage>,
    temperature: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<NativeToolSpec>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_choice: Option<String>,
}

#[derive(Debug, Serialize)]
struct NativeMessage {
    role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_call_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Vec<NativeToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reasoning_content: Option<String>,
}

#[derive(Debug, Serialize)]
struct NativeToolSpec {
    #[serde(rename = "type")]
    kind: String,
    function: NativeToolFunctionSpec,
}

#[derive(Debug, Serialize)]
struct NativeToolFunctionSpec {
    name: String,
    description: String,
    parameters: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
struct NativeToolCall {
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<String>,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    kind: Option<String>,
    function: NativeFunctionCall,
}

#[derive(Debug, Serialize, Deserialize)]
struct NativeFunctionCall {
    name: String,
    arguments: String,
}

#[derive(Debug, Deserialize)]
struct NativeChatResponse {
    choices: Vec<NativeChoice>,
    #[serde(default)]
    usage: Option<UsageInfo>,
}

#[derive(Debug, Deserialize)]
struct UsageInfo {
    #[serde(default)]
    prompt_tokens: Option<u64>,
    #[serde(default)]
    completion_tokens: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct NativeChoice {
    message: NativeResponseMessage,
}

#[derive(Debug, Deserialize)]
struct NativeResponseMessage {
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    reasoning_content: Option<String>,
    #[serde(default)]
    tool_calls: Option<Vec<NativeToolCall>>,
}

impl CompatibleProvider {
    fn parse_native_response(message: NativeResponseMessage) -> ProviderChatResponse {
        let reasoning_content = message.reasoning_content.clone();
        let tool_calls = message
            .tool_calls
            .unwrap_or_default()
            .into_iter()
            .map(|tc| ProviderToolCall {
                id: tc.id.unwrap_or_default(),
                name: tc.function.name,
                arguments: tc.function.arguments,
            })
            .collect::<Vec<_>>();
        ProviderChatResponse {
            text: message.content,
            tool_calls,
            usage: None,
            reasoning_content,
            quota_metadata: None,
        }
    }
}

#[async_trait]
impl Provider for CompatibleProvider {
    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            native_tool_calling: self.native_tool_calling,
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
        let credential_check = self
            .credential
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("{} API key not set.", self.name))?;
        let _ = credential_check;

        let mut messages = Vec::new();
        if let Some(sys) = system_prompt {
            messages.push(SimpleMessage {
                role: "system".to_string(),
                content: sys.to_string(),
            });
        }
        messages.push(SimpleMessage {
            role: "user".to_string(),
            content: message.to_string(),
        });

        let request = SimpleChatRequest {
            model: model.to_string(),
            messages,
            temperature,
            max_tokens: self.max_tokens_override,
        };

        let req = self.add_auth_header(self.client.post(self.chat_url()).json(&request));
        let response = req.send().await?;

        if !response.status().is_success() {
            return Err(api_error(&self.name, response).await);
        }

        let chat_response: SimpleChatResponse = response.json().await?;
        chat_response
            .choices
            .into_iter()
            .next()
            .map(|c| c.message.content)
            .ok_or_else(|| anyhow::anyhow!("No response from {}", self.name))
    }

    async fn chat(
        &self,
        request: ProviderChatRequest<'_>,
        model: &str,
        temperature: f64,
    ) -> anyhow::Result<ProviderChatResponse> {
        let _ = self
            .credential
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("{} API key not set.", self.name))?;

        let tools = Self::convert_tools(request.tools);
        let native_request = NativeChatRequest {
            model: model.to_string(),
            messages: Self::convert_messages(request.messages),
            temperature,
            max_tokens: self.max_tokens_override,
            tool_choice: tools.as_ref().map(|_| "auto".to_string()),
            tools,
        };

        let req = self.add_auth_header(self.client.post(self.chat_url()).json(&native_request));
        let response = req.send().await?;

        if !response.status().is_success() {
            return Err(api_error(&self.name, response).await);
        }

        let native_response: NativeChatResponse = response.json().await?;
        let usage = native_response.usage.map(|u| TokenUsage {
            input_tokens: u.prompt_tokens,
            output_tokens: u.completion_tokens,
        });

        let message = native_response
            .choices
            .into_iter()
            .next()
            .map(|c| c.message)
            .ok_or_else(|| anyhow::anyhow!("No response from {}", self.name))?;

        let mut result = Self::parse_native_response(message);
        result.usage = usage;
        Ok(result)
    }
}

/// 常用兼容服务的预设 base_url。
pub mod presets {
    /// DeepSeek
    pub const DEEPSEEK_BASE_URL: &str = "https://api.deepseek.com/v1";
    /// Kimi / Moonshot
    pub const MOONSHOT_BASE_URL: &str = "https://api.moonshot.cn/v1";
    /// Minimax
    pub const MINIMAX_BASE_URL: &str = "https://api.minimax.chat/v1";
    /// 通义千问（兼容模式）
    pub const QWEN_BASE_URL: &str = "https://dashscope.aliyuncs.com/compatible-mode/v1";
    /// Groq
    pub const GROQ_BASE_URL: &str = "https://api.groq.com/openai/v1";
    /// Mistral
    pub const MISTRAL_BASE_URL: &str = "https://api.mistral.ai/v1";
    /// xAI (Grok)
    pub const XAI_BASE_URL: &str = "https://api.x.ai/v1";
    /// 智谱 GLM（OpenAI 兼容入口，非 JWT）
    pub const GLM_BASE_URL: &str = "https://open.bigmodel.cn/api/paas/v4";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_provider() {
        let p = CompatibleProvider::new("deepseek", presets::DEEPSEEK_BASE_URL, Some("key"));
        assert_eq!(p.name, "deepseek");
        assert_eq!(p.base_url, presets::DEEPSEEK_BASE_URL);
        assert_eq!(p.credential.as_deref(), Some("key"));
    }

    #[test]
    fn chat_url_correct() {
        let p = CompatibleProvider::new("test", "https://api.example.com/v1", None);
        assert_eq!(p.chat_url(), "https://api.example.com/v1/chat/completions");
    }

    #[test]
    fn chat_url_trims_trailing_slash() {
        let p = CompatibleProvider::new("test", "https://api.example.com/v1/", None);
        assert_eq!(p.chat_url(), "https://api.example.com/v1/chat/completions");
    }

    #[test]
    fn auth_style_default_bearer() {
        let p = CompatibleProvider::new("t", "https://x", None);
        assert_eq!(p.auth_style, AuthStyle::Bearer);
    }

    #[test]
    fn convert_tools_handles_empty() {
        assert!(CompatibleProvider::convert_tools(None).is_none());
        assert!(CompatibleProvider::convert_tools(Some(&[])).is_none());
    }

    #[test]
    fn convert_tools_builds_spec() {
        let tools = vec![ToolSpec {
            name: "shell".to_string(),
            description: "run".to_string(),
            parameters: serde_json::json!({"type": "object"}),
        }];
        let native = CompatibleProvider::convert_tools(Some(&tools)).unwrap();
        assert_eq!(native[0].function.name, "shell");
    }

    #[test]
    fn convert_messages_plain() {
        let msgs = vec![ChatMessage::user("hi")];
        let native = CompatibleProvider::convert_messages(&msgs);
        assert_eq!(native[0].role, "user");
        assert_eq!(native[0].content.as_deref(), Some("hi"));
    }

    #[test]
    fn presets_are_valid_urls() {
        assert!(presets::DEEPSEEK_BASE_URL.starts_with("https://"));
        assert!(presets::MOONSHOT_BASE_URL.contains("moonshot"));
        assert!(presets::QWEN_BASE_URL.contains("dashscope"));
    }

    #[test]
    fn with_native_tool_calling_disabled() {
        let p = CompatibleProvider::new("t", "https://x", None).with_native_tool_calling(false);
        assert!(!p.capabilities().native_tool_calling);
    }
}
