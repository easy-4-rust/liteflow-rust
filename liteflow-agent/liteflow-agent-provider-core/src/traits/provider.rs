// Copyright 2025 ZeroClaw Labs.
// Modified by LiteFlow-Rust contributors.
// Source: https://github.com/zeroclaw-labs/zeroclaw (Apache-2.0)
// SPDX-License-Identifier: Apache-2.0

use super::{
    ChatMessage, ChatRequest, ChatResponse, ProviderCapabilities, StreamChunk, StreamOptions,
    StreamResult, ToolsPayload, build_tool_instructions_text,
};
use crate::tool_spec::ToolSpec;
use async_trait::async_trait;
use futures_util::{StreamExt, stream};

/// 统一不同大模型平台的聊天、工具调用、能力声明与流式响应接口。
///
/// 默认实现提供非原生工具调用的提示词降级和历史消息适配；具体平台只需覆盖其
/// 支持的能力。对应 Java: 无（Rust 提供商基础设施；源自 ZeroClaw `Provider`）。
#[async_trait]
pub trait Provider: Send + Sync {
    /// 查询提供商能力。
    ///
    /// # 返回
    /// 默认返回不支持原生工具调用和视觉输入的最小能力集。
    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities::default()
    }

    /// 将统一工具定义转换为提供商载荷。
    ///
    /// # 参数
    /// - `tools`: 统一工具定义。
    ///
    /// # 返回
    /// 默认返回可注入系统提示词的说明文本。
    fn convert_tools(&self, tools: &[ToolSpec]) -> ToolsPayload {
        ToolsPayload::PromptGuided {
            instructions: build_tool_instructions_text(tools),
        }
    }

    /// 执行不含显式系统提示词的单轮聊天。
    ///
    /// # 参数
    /// - `message`: 用户消息。
    /// - `model`: 模型名称。
    /// - `temperature`: 采样温度。
    ///
    /// # 返回
    /// 提供商生成的文本或调用错误。
    async fn simple_chat(
        &self,
        message: &str,
        model: &str,
        temperature: f64,
    ) -> anyhow::Result<String> {
        self.chat_with_system(None, message, model, temperature)
            .await
    }

    /// 执行可选系统提示词的单轮聊天。
    ///
    /// # 参数
    /// - `system_prompt`: 可选系统提示词。
    /// - `message`: 用户消息。
    /// - `model`: 模型名称。
    /// - `temperature`: 采样温度。
    async fn chat_with_system(
        &self,
        system_prompt: Option<&str>,
        message: &str,
        model: &str,
        temperature: f64,
    ) -> anyhow::Result<String>;

    /// 执行多轮聊天；默认提取首个系统消息和最后一个用户消息后降级为单轮调用。
    ///
    /// # 参数
    /// - `messages`: 有序历史消息。
    /// - `model`: 模型名称。
    /// - `temperature`: 采样温度。
    async fn chat_with_history(
        &self,
        messages: &[ChatMessage],
        model: &str,
        temperature: f64,
    ) -> anyhow::Result<String> {
        let system = messages
            .iter()
            .find(|message| message.role == "system")
            .map(|message| message.content.as_str());
        let last_user = messages
            .iter()
            .rfind(|message| message.role == "user")
            .map(|message| message.content.as_str())
            .unwrap_or("");
        self.chat_with_system(system, last_user, model, temperature)
            .await
    }

    /// 执行可携带工具定义的结构化聊天。
    ///
    /// 当提供商不支持原生工具调用时，将工具说明合并到已有系统消息，或在消息序列
    /// 前新增系统消息。对应 Java: 无；源自 ZeroClaw `Provider#chat`。
    ///
    /// # 参数
    /// - `request`: 借用的消息与工具定义。
    /// - `model`: 模型名称。
    /// - `temperature`: 采样温度。
    async fn chat(
        &self,
        request: ChatRequest<'_>,
        model: &str,
        temperature: f64,
    ) -> anyhow::Result<ChatResponse> {
        if let Some(tools) = request.tools
            && !tools.is_empty()
            && !self.supports_native_tools()
        {
            let tool_instructions = match self.convert_tools(tools) {
                ToolsPayload::PromptGuided { instructions } => instructions,
                payload => anyhow::bail!(
                    "Provider returned non-prompt-guided tools payload ({payload:?}) while supports_native_tools() is false"
                ),
            };
            let mut modified_messages = request.messages.to_vec();

            // 优先扩展已有系统消息，避免制造多个互相竞争的 system 角色消息。
            if let Some(system_message) = modified_messages
                .iter_mut()
                .find(|message| message.role == "system")
            {
                if !system_message.content.is_empty() {
                    system_message.content.push_str("\n\n");
                }
                system_message.content.push_str(&tool_instructions);
            } else {
                modified_messages.insert(0, ChatMessage::system(tool_instructions));
            }

            let text = self
                .chat_with_history(&modified_messages, model, temperature)
                .await?;
            return Ok(ChatResponse {
                text: Some(text),
                tool_calls: Vec::new(),
                usage: None,
                reasoning_content: None,
                quota_metadata: None,
            });
        }

        let text = self
            .chat_with_history(request.messages, model, temperature)
            .await?;
        Ok(ChatResponse {
            text: Some(text),
            tool_calls: Vec::new(),
            usage: None,
            reasoning_content: None,
            quota_metadata: None,
        })
    }

    /// 使用所有权消息执行结构化聊天，桥接要求 `'static` 生命周期的流式模型接口。
    ///
    /// # 参数
    /// - `messages`: 自有消息序列。
    /// - `tools`: 可选自有工具定义。
    /// - `model`: 模型名称。
    /// - `temperature`: 采样温度。
    ///
    /// # 返回
    /// 结构化聊天响应。
    ///
    /// 这是 LiteFlow-Rust 为 AgentScope `Model::stream` 新增的桥接方法。
    async fn chat_owned(
        &self,
        messages: Vec<ChatMessage>,
        tools: Option<Vec<ToolSpec>>,
        model: &str,
        temperature: f64,
    ) -> anyhow::Result<ChatResponse> {
        let request = ChatRequest {
            messages: &messages,
            tools: tools.as_deref(),
        };
        self.chat(request, model, temperature).await
    }

    /// 判断提供商是否支持 API 原生工具调用。
    fn supports_native_tools(&self) -> bool {
        self.capabilities().native_tool_calling
    }

    /// 判断提供商是否支持视觉输入。
    fn supports_vision(&self) -> bool {
        self.capabilities().vision
    }

    /// 预热提供商连接池。
    ///
    /// 默认实现不主动建立连接；持有 HTTP 客户端的实现可覆盖此方法。
    async fn warmup(&self) -> anyhow::Result<()> {
        Ok(())
    }

    /// 使用提供商原生工具定义执行聊天。
    ///
    /// 默认实现忽略原生载荷并回退到历史聊天，返回不含工具调用的结构化响应。
    ///
    /// # 参数
    /// - `messages`: 有序历史消息。
    /// - `_tools`: 提供商原生 JSON 工具定义。
    /// - `model`: 模型名称。
    /// - `temperature`: 采样温度。
    async fn chat_with_tools(
        &self,
        messages: &[ChatMessage],
        _tools: &[serde_json::Value],
        model: &str,
        temperature: f64,
    ) -> anyhow::Result<ChatResponse> {
        let text = self.chat_with_history(messages, model, temperature).await?;
        Ok(ChatResponse {
            text: Some(text),
            tool_calls: Vec::new(),
            usage: None,
            reasoning_content: None,
            quota_metadata: None,
        })
    }

    /// 判断提供商是否支持流式响应；默认返回 `false`。
    fn supports_streaming(&self) -> bool {
        false
    }

    /// 执行可选系统提示词的流式聊天。
    ///
    /// 默认实现返回空流，表示当前提供商没有声明流式能力。
    ///
    /// # 参数
    /// - `_system_prompt`: 可选系统提示词。
    /// - `_message`: 用户消息。
    /// - `_model`: 模型名称。
    /// - `_temperature`: 采样温度。
    /// - `_options`: 流式选项。
    fn stream_chat_with_system(
        &self,
        _system_prompt: Option<&str>,
        _message: &str,
        _model: &str,
        _temperature: f64,
        _options: StreamOptions,
    ) -> stream::BoxStream<'static, StreamResult<StreamChunk>> {
        stream::empty().boxed()
    }

    /// 执行多轮流式聊天；默认按非流式历史语义提取系统消息和最后一个用户消息。
    ///
    /// # 参数
    /// - `messages`: 有序历史消息。
    /// - `model`: 模型名称。
    /// - `temperature`: 采样温度。
    /// - `options`: 流式选项。
    fn stream_chat_with_history(
        &self,
        messages: &[ChatMessage],
        model: &str,
        temperature: f64,
        options: StreamOptions,
    ) -> stream::BoxStream<'static, StreamResult<StreamChunk>> {
        let system = messages
            .iter()
            .find(|message| message.role == "system")
            .map(|message| message.content.as_str());
        let last_user = messages
            .iter()
            .rfind(|message| message.role == "user")
            .map(|message| message.content.as_str())
            .unwrap_or("");
        self.stream_chat_with_system(system, last_user, model, temperature, options)
    }
}
