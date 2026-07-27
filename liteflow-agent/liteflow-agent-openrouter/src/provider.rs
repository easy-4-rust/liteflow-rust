// Copyright 2025 ZeroClaw Labs.
// Modified by LiteFlow-Rust contributors.
// Source: https://github.com/zeroclaw-labs/zeroclaw (Apache-2.0)
// SPDX-License-Identifier: Apache-2.0
//
// 本文件衍生自 ZeroClaw 项目 src/providers/openrouter.rs。
// 修改：
// - import 路径调整为 liteflow-agent-provider-core；
// - 精简：去掉 multimodal 图片标记解析（image marker），仅保留文本 + native tool calling；
// - HTTP-Referer/X-Title 改为 LiteFlow-Rust 归属。
// "ZeroClaw" 是 ZeroClaw Labs 的商标；本项目与其无官方关联。

//! OpenRouter 聚合网关 provider。
//!
//! 通过 OpenAI 兼容协议接入 100+ 模型，支持 native tool calling。

use async_trait::async_trait;
use liteflow_agent_provider_core::util::api_error;
use liteflow_agent_provider_core::{
    ChatMessage, ChatRequest as ProviderChatRequest, ChatResponse as ProviderChatResponse,
    Provider, ProviderCapabilities, TokenUsage, ToolCall as ProviderToolCall, ToolSpec,
};
use reqwest::Client;
use serde::{Deserialize, Serialize};

const BASE_URL: &str = "https://openrouter.ai/api/v1/chat/completions";

pub struct OpenRouterProvider {
    credential: Option<String>,
    max_tokens_override: Option<u32>,
    client: Client,
}

#[derive(Debug, Serialize)]
struct ChatRequest {
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
struct ApiChatResponse {
    choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: ResponseMessage,
}

#[derive(Debug, Deserialize)]
struct ResponseMessage {
    content: String,
}

// ── Native tool calling 类型 ──

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

impl OpenRouterProvider {
    /// 创建 OpenRouter provider。
    pub fn new(credential: Option<&str>) -> Self {
        Self::new_with_max_tokens(credential, None)
    }

    /// 创建带 max_tokens 覆盖的 provider。
    pub fn new_with_max_tokens(credential: Option<&str>, max_tokens_override: Option<u32>) -> Self {
        Self {
            credential: credential.map(ToString::to_string),
            max_tokens_override: max_tokens_override.filter(|value| *value > 0),
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(120))
                .connect_timeout(std::time::Duration::from_secs(10))
                .build()
                .unwrap_or_else(|_| Client::new()),
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

    /// 把 ChatMessage 转为 native 消息（处理 assistant 带 tool_calls / tool 角色结果）。
    fn convert_messages(messages: &[ChatMessage]) -> Vec<NativeMessage> {
        messages
            .iter()
            .map(|m| {
                if m.role == "assistant" {
                    // assistant content 可能是 JSON（含 tool_calls）
                    if let Ok(value) = serde_json::from_str::<serde_json::Value>(&m.content) {
                        if let Some(tool_calls_value) = value.get("tool_calls") {
                            if let Ok(parsed_calls) =
                                serde_json::from_value::<Vec<ProviderToolCall>>(
                                    tool_calls_value.clone(),
                                )
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
                    // tool 角色消息 content 可能是 JSON（含 tool_call_id + content）
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
impl Provider for OpenRouterProvider {
    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            native_tool_calling: true,
            vision: false, // 精简版不支持图片
        }
    }

    async fn chat_with_system(
        &self,
        system_prompt: Option<&str>,
        message: &str,
        model: &str,
        temperature: f64,
    ) -> anyhow::Result<String> {
        let credential = self.credential.as_ref().ok_or_else(|| {
            anyhow::anyhow!("OpenRouter API key not set. Set OPENROUTER_API_KEY env var.")
        })?;

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

        let request = ChatRequest {
            model: model.to_string(),
            messages,
            temperature,
            max_tokens: self.max_tokens_override,
        };

        let response = self
            .client
            .post(BASE_URL)
            .header("Authorization", format!("Bearer {credential}"))
            .header("HTTP-Referer", "https://github.com/dromara/liteflow")
            .header("X-Title", "LiteFlow-Rust")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(api_error("OpenRouter", response).await);
        }

        let chat_response: ApiChatResponse = response.json().await?;
        chat_response
            .choices
            .into_iter()
            .next()
            .map(|c| c.message.content)
            .ok_or_else(|| anyhow::anyhow!("No response from OpenRouter"))
    }

    async fn chat(
        &self,
        request: ProviderChatRequest<'_>,
        model: &str,
        temperature: f64,
    ) -> anyhow::Result<ProviderChatResponse> {
        let credential = self.credential.as_ref().ok_or_else(|| {
            anyhow::anyhow!("OpenRouter API key not set. Set OPENROUTER_API_KEY env var.")
        })?;

        let tools = Self::convert_tools(request.tools);
        let native_request = NativeChatRequest {
            model: model.to_string(),
            messages: Self::convert_messages(request.messages),
            temperature,
            max_tokens: self.max_tokens_override,
            tool_choice: tools.as_ref().map(|_| "auto".to_string()),
            tools,
        };

        let response = self
            .client
            .post(BASE_URL)
            .header("Authorization", format!("Bearer {credential}"))
            .header("HTTP-Referer", "https://github.com/dromara/liteflow")
            .header("X-Title", "LiteFlow-Rust")
            .json(&native_request)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(api_error("OpenRouter", response).await);
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
            .ok_or_else(|| anyhow::anyhow!("No response from OpenRouter"))?;

        let mut result = Self::parse_native_response(message);
        result.usage = usage;
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_provider() {
        let p = OpenRouterProvider::new(Some("test-key"));
        assert_eq!(p.credential.as_deref(), Some("test-key"));
    }

    #[test]
    fn convert_tools_handles_empty() {
        assert!(OpenRouterProvider::convert_tools(None).is_none());
        assert!(OpenRouterProvider::convert_tools(Some(&[])).is_none());
    }

    #[test]
    fn convert_tools_builds_native_spec() {
        let tools = vec![ToolSpec {
            name: "shell".to_string(),
            description: "run cmd".to_string(),
            parameters: serde_json::json!({"type": "object"}),
        }];
        let native = OpenRouterProvider::convert_tools(Some(&tools)).unwrap();
        assert_eq!(native.len(), 1);
        assert_eq!(native[0].function.name, "shell");
    }

    #[test]
    fn convert_messages_plain_text() {
        let msgs = vec![ChatMessage::system("be helpful"), ChatMessage::user("hi")];
        let native = OpenRouterProvider::convert_messages(&msgs);
        assert_eq!(native.len(), 2);
        assert_eq!(native[0].role, "system");
        assert_eq!(native[1].content.as_deref(), Some("hi"));
    }
}
