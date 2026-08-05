use agentscope_core::ContentBlock;
use agentscope_core::agent::AgentError as AgentScopeError;
use agentscope_core::hook::{Hook, HookEvent};
use agentscope_core::message::{ToolResultBlock, ToolUseBlock};
use async_trait::async_trait;
use std::sync::atomic::{AtomicBool, Ordering};

/// 记录 ReAct 推理、工具调用和错误生命周期的低优先级 Hook。
///
/// 日志内容会折叠空白并限制长度，避免模型思考、工具参数或执行结果无限放大日志。
/// 格式化过程不改变原事件，也不会阻断后续 Hook。
///
/// 对应 Java: `com.yomahub.liteflow.agent.hook.ReActLoggingHook`。
pub struct ReActLoggingHook {
    session_id: String,
    reasoning_events_observed: AtomicBool,
}

impl ReActLoggingHook {
    const MAX_TEXT_LEN: usize = 500;
    const MAX_THINKING_LEN: usize = 1_000;
    const MAX_RESULT_LEN: usize = 2_000;

    /// 创建指定会话的 ReAct 日志 Hook。
    ///
    /// # 参数
    /// - `session_id`: 日志关联标识；空白值会统一显示为 `-`。
    ///
    /// # 返回
    /// 可注册到 AgentScope ReActAgent 的日志 Hook。
    ///
    /// 对应 Java: `ReActLoggingHook#ReActLoggingHook`。
    #[must_use]
    pub fn new(session_id: impl Into<String>) -> Self {
        let session_id = session_id.into();
        Self {
            session_id: if session_id.trim().is_empty() {
                "-".to_string()
            } else {
                session_id
            },
            reasoning_events_observed: AtomicBool::new(false),
        }
    }

    /// 记录一次 Agent 调用的推理入口。
    ///
    /// # 参数
    /// - `message_count`: 本次调用传入 Agent 的用户消息数量。
    ///
    /// 对应 Java: `ReActLoggingHook` 的 `PreReasoningEvent` 日志。
    pub(crate) fn log_reasoning_start(&self, message_count: usize) {
        self.reasoning_events_observed
            .store(true, Ordering::Release);
        tracing::info!(
            "[agent:reason][{}] >>> messages={message_count}",
            self.session_id
        );
    }

    /// 记录 Agent 最终推理消息。
    ///
    /// # 参数
    /// - `message`: 最终回答、思考摘要、工具调用和 usage 的载体。
    ///
    /// 对应 Java: `ReActLoggingHook` 的 `PostReasoningEvent` 日志。
    pub(crate) fn log_final_message(&self, message: &agentscope_core::Msg) {
        self.log_reasoning_message(message);
    }

    /// 记录 Agent 调用失败。
    ///
    /// # 参数
    /// - `message`: AgentScope 返回的错误文本。
    ///
    /// 对应 Java: `ReActLoggingHook` 的 `ErrorEvent` 日志。
    pub(crate) fn log_execution_error(&self, message: &str) {
        tracing::warn!(
            "[agent:error][{}] phase=CALL, type=AgentScopeError, message={}",
            self.session_id,
            Self::truncate(message, Self::MAX_RESULT_LEN)
        );
    }

    fn log_event(&self, event: &HookEvent) {
        match event {
            HookEvent::PreCall(data) => {
                self.log_reasoning_start(data.input_messages.len());
            }
            HookEvent::PreReasoning(data) => {
                self.reasoning_events_observed
                    .store(true, Ordering::Release);
                tracing::info!(
                    "[agent:reason][{}] >>> model={}, messages={}",
                    self.session_id,
                    data.model_name,
                    data.input_messages.len()
                );
            }
            HookEvent::PostReasoning(data) => {
                let Some(message) = data.reasoning_message.as_ref() else {
                    return;
                };
                self.log_reasoning_message(message);
            }
            HookEvent::PostCall(data)
                if !self.reasoning_events_observed.load(Ordering::Acquire) =>
            {
                // 当前 AgentScope-Rust 2.0 主链尚未分发 reasoning 事件；使用最终消息
                // 作为兼容回退，确保 logging.react_enabled 在真实执行中确实可观测。
                self.log_reasoning_message(&data.final_message);
            }
            HookEvent::PreActing(data) => {
                tracing::info!(
                    "[agent:act][{}] >>> tool={}, input={}",
                    self.session_id,
                    data.tool_use.name,
                    Self::tool_input(&data.tool_use)
                );
            }
            HookEvent::PostActing(data) => {
                let result = data
                    .tool_result
                    .as_ref()
                    .map_or_else(|| "-".to_string(), Self::tool_result);
                tracing::info!(
                    "[agent:act][{}] <<< tool={}, result={}",
                    self.session_id,
                    data.tool_use.name,
                    result
                );
            }
            HookEvent::Error(data) => {
                tracing::warn!(
                    "[agent:error][{}] phase={:?}, type={}, message={}",
                    self.session_id,
                    data.phase,
                    data.error_type,
                    Self::truncate(&data.error_message, Self::MAX_RESULT_LEN)
                );
            }
            _ => {}
        }
    }

    fn log_reasoning_message(&self, message: &agentscope_core::Msg) {
        for thinking in message.content().iter().filter_map(|block| match block {
            ContentBlock::Thinking(thinking) => Some(thinking.thinking.as_str()),
            _ => None,
        }) {
            tracing::info!(
                "[agent:reason][{}] thinking={}",
                self.session_id,
                Self::truncate(thinking, Self::MAX_THINKING_LEN)
            );
        }
        let text = message.get_text_content();
        if !text.trim().is_empty() {
            tracing::info!(
                "[agent:reason][{}] text={}",
                self.session_id,
                Self::truncate(&text, Self::MAX_TEXT_LEN)
            );
        }
        for tool_use in message.tool_use_blocks() {
            tracing::info!(
                "[agent:reason][{}] tool={}, input={}",
                self.session_id,
                tool_use.name,
                Self::tool_input(tool_use)
            );
        }
        if let Some(usage) = message.get_chat_usage() {
            tracing::info!(
                "[agent:reason][{}] usage input={}, output={}, total={}, time={:.3}s",
                self.session_id,
                usage.input_tokens,
                usage.output_tokens,
                usage.total_tokens(),
                usage.time
            );
        }
    }

    fn tool_input(tool_use: &ToolUseBlock) -> String {
        let input = serde_json::to_string(&tool_use.input)
            .unwrap_or_else(|error| format!("<serialize-error:{error}>"));
        Self::truncate(&input, Self::MAX_TEXT_LEN)
    }

    fn tool_result(tool_result: &ToolResultBlock) -> String {
        let output = tool_result
            .output
            .iter()
            .map(|block| match block {
                ContentBlock::Text(text) => text.text.clone(),
                ContentBlock::Thinking(thinking) => thinking.thinking.clone(),
                other => serde_json::to_string(other)
                    .unwrap_or_else(|error| format!("<serialize-error:{error}>")),
            })
            .collect::<Vec<_>>()
            .join("\n");
        Self::truncate(&output, Self::MAX_RESULT_LEN)
    }

    fn truncate(value: &str, max_chars: usize) -> String {
        let normalized = value.split_whitespace().collect::<Vec<_>>().join(" ");
        if normalized.chars().count() <= max_chars {
            return normalized;
        }
        let prefix = normalized.chars().take(max_chars).collect::<String>();
        format!("{prefix}...(truncated)")
    }
}

#[async_trait]
impl Hook for ReActLoggingHook {
    async fn on_event(&self, event: HookEvent) -> Result<HookEvent, AgentScopeError> {
        // 日志 Hook 只观察事件；即使某个字段无法序列化，也输出降级文本并透传事件。
        self.log_event(&event);
        Ok(event)
    }

    fn priority(&self) -> i32 {
        900
    }
}

#[cfg(test)]
mod tests {
    use agentscope_core::hook::Hook;
    use agentscope_core::message::ToolUseBlock;
    use serde_json::json;
    use std::collections::HashMap;
    use std::sync::atomic::Ordering;

    use super::ReActLoggingHook;

    #[test]
    fn truncation_collapses_whitespace_and_preserves_unicode_boundaries() {
        assert_eq!(
            ReActLoggingHook::truncate("  你好\n 世界  ", 4),
            "你好 世...(truncated)"
        );
        assert_eq!(ReActLoggingHook::truncate("a \n b", 10), "a b");
    }

    #[test]
    fn tool_input_is_json_and_bounded() {
        let tool_use = ToolUseBlock::new(
            "call-1",
            "search",
            HashMap::from([("query".to_string(), json!("rust"))]),
        );
        let input = ReActLoggingHook::tool_input(&tool_use);
        assert!(input.contains("\"query\":\"rust\""));
        assert!(input.chars().count() <= ReActLoggingHook::MAX_TEXT_LEN);
    }

    #[test]
    fn blank_session_and_java_priority_are_preserved() {
        let hook = ReActLoggingHook::new("  ");
        assert_eq!(hook.session_id, "-");
        assert!(!hook.reasoning_events_observed.load(Ordering::Acquire));
        assert_eq!(hook.priority(), 900);
    }
}
