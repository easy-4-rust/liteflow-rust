// SPDX-License-Identifier: Apache-2.0
//
// LiteFlow-Rust 原创代码：把 zeroclaw `Provider` 包装成 agentscope `Model`。
// 转换逻辑基于 agentscope-rust 的实际 API（ContentBlock::ToolUse、Msg::content() 等）。

//! `ProviderToModelAdapter` — 把任意 zeroclaw `Provider` 包装成 agentscope `Model`。
//!
//! ## 转换逻辑
//!
//! | agentscope 类型 | zeroclaw 类型 | 说明 |
//! |---|---|---|
//! | `Msg` | `ChatMessage` | 提取 role + 文本 content（多模态暂不支持） |
//! | `ToolSchema` | `ToolSpec` | 字段一一对应（name/description/parameters） |
//! | `ChatResponse`（agentscope） | `ChatResponse`（zeroclaw） | text→`TextBlock`，tool_calls→`ToolUse` |
//!
//! 单次 `Provider::chat()` 结果包装成单元素 `Stream`，满足 `Model::stream()` 签名。

use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;

use agentscope_core::message::{TextBlock, ToolUseBlock};
use agentscope_core::model::ToolSchema;
use agentscope_core::model::{ChatResponse as AsChatResponse, GenerateOptions, ModelError};
use agentscope_core::{ContentBlock, Model, Msg, MsgRole};
use futures_util::stream::{self, Stream};
use serde_json::Value;

use crate::tool_spec::ToolSpec;
use crate::traits::{ChatMessage, ChatResponse as ZcChatResponse, Provider};

/// 任意 `Provider` 的 `Model` 适配器。
///
/// 把 zeroclaw 的同步/单次请求 `Provider` 适配为 agentscope 的流式 `Model`，
/// 使 GLM/Copilot/Bedrock 等 zeroclaw 独有平台能接入 `ReActAgentComponent`。
pub struct ProviderToModelAdapter {
    name: String,
    model_name: String,
    provider: Arc<dyn Provider>,
    temperature: f64,
}

impl ProviderToModelAdapter {
    /// 构造适配器。
    ///
    /// - `name`：显示名（如 "glm-4.6"）
    /// - `model_name`：传给 Provider 的模型 ID
    /// - `provider`：zeroclaw Provider 实例（如 `GlmProvider::new(...)`）
    /// - `temperature`：默认采样温度
    #[must_use]
    pub fn new(
        name: impl Into<String>,
        model_name: impl Into<String>,
        provider: Arc<dyn Provider>,
        temperature: f64,
    ) -> Self {
        Self {
            name: name.into(),
            model_name: model_name.into(),
            provider,
            temperature,
        }
    }
}

impl Model for ProviderToModelAdapter {
    fn name(&self) -> &str {
        &self.name
    }

    fn stream(
        &self,
        messages: &[Msg],
        tools: &[ToolSchema],
        _options: Option<&GenerateOptions>,
    ) -> Pin<Box<dyn Stream<Item = Result<AsChatResponse, ModelError>> + Send>> {
        // 1. Msg → ChatMessage（owned，满足 'static）
        let zc_messages: Vec<ChatMessage> = messages.iter().map(msg_to_chat_message).collect();

        // 2. ToolSchema → ToolSpec
        let zc_tools: Vec<ToolSpec> = tools.iter().map(tool_schema_to_spec).collect();

        // 3. Clone 参数进 async block
        let provider = Arc::clone(&self.provider);
        let model_name = self.model_name.clone();
        let temperature = self.temperature;

        // 4. 调用 Provider::chat_owned()，把结果包成单元素 Stream
        Box::pin(stream::once(async move {
            let tools_opt = if zc_tools.is_empty() {
                None
            } else {
                Some(zc_tools)
            };
            let result = provider
                .chat_owned(zc_messages, tools_opt, &model_name, temperature)
                .await
                .map_err(|e| ModelError::Other(format!("{e:?}")))?;
            Ok(zc_response_to_as_response(result, &model_name))
        }))
    }
}

/// `Msg` → `ChatMessage`：提取 role + 纯文本 content。
///
/// 注意：multimodal content（图片/音频/视频）暂不支持，仅提取 `TextBlock`。
/// 这是已知限制，后续如需多模态可扩展为内联 base64。
fn msg_to_chat_message(msg: &Msg) -> ChatMessage {
    let role = match msg.role() {
        MsgRole::System => "system",
        MsgRole::User => "user",
        MsgRole::Assistant => "assistant",
        MsgRole::Tool => "tool",
    };
    // Msg::content() 返回 &[ContentBlock]，提取所有 Text 块拼接
    let content: String = msg
        .content()
        .iter()
        .filter_map(|b| match b {
            ContentBlock::Text(t) => Some(t.text.as_str()),
            _ => None, // 图片/音频等暂跳过（已知限制）
        })
        .collect::<Vec<_>>()
        .join("");
    ChatMessage {
        role: role.to_string(),
        content,
    }
}

/// `ToolSchema` → `ToolSpec`（字段一一对应）。
fn tool_schema_to_spec(schema: &ToolSchema) -> ToolSpec {
    ToolSpec {
        name: schema.name.clone(),
        description: schema.description.clone(),
        parameters: schema.parameters.clone(),
    }
}

/// zeroclaw `ChatResponse` → agentscope `ChatResponse`。
fn zc_response_to_as_response(zc: ZcChatResponse, model_name: &str) -> AsChatResponse {
    let mut content = Vec::new();
    if let Some(text) = zc.text {
        if !text.is_empty() {
            content.push(ContentBlock::Text(TextBlock { text }));
        }
    }
    for tc in zc.tool_calls {
        // zeroclaw ToolCall.arguments 是 String(JSON)，agentscope ToolUseBlock.input 是 HashMap
        let input: HashMap<String, Value> = serde_json::from_str(&tc.arguments).unwrap_or_default();
        content.push(ContentBlock::ToolUse(ToolUseBlock::new(
            tc.id, tc.name, input,
        )));
    }
    AsChatResponse::builder()
        .content(content)
        .model_name(model_name.to_string())
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_util::StreamExt;

    /// 一个简单的测试用 Provider：固定返回 "pong"。
    struct EchoProvider;

    #[async_trait::async_trait]
    impl Provider for EchoProvider {
        async fn chat_with_system(
            &self,
            _system_prompt: Option<&str>,
            _message: &str,
            _model: &str,
            _temperature: f64,
        ) -> anyhow::Result<String> {
            Ok("pong".to_string())
        }
    }

    #[tokio::test]
    async fn adapter_wraps_provider_into_model() {
        let provider = Arc::new(EchoProvider);
        let model = ProviderToModelAdapter::new("test-echo", "echo-model", provider, 0.7);

        assert_eq!(model.name(), "test-echo");

        let msg = Msg::builder()
            .role(MsgRole::User)
            .text_content("ping")
            .build();
        let mut stream = model.stream(&[msg], &[], None);
        let resp = stream.next().await.expect("stream should yield one item");
        let resp = resp.expect("response should be ok");
        assert_eq!(resp.model_name.as_deref(), Some("echo-model"));
        let text = resp.text();
        assert_eq!(text, "pong");
    }

    #[test]
    fn msg_to_chat_message_extracts_text() {
        let msg = Msg::builder()
            .role(MsgRole::User)
            .text_content("hello")
            .build();
        let cm = msg_to_chat_message(&msg);
        assert_eq!(cm.role, "user");
        assert_eq!(cm.content, "hello");
    }
}
