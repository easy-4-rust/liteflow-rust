use std::sync::Mutex;

use agentscope_core::agent::AgentError as AgentScopeError;
use agentscope_core::hook::{Hook, HookEvent};
use agentscope_core::message::ChatUsage;
use async_trait::async_trait;

#[derive(Default)]
struct UsageState {
    input_tokens: i32,
    output_tokens: i32,
    cached_tokens: i32,
    time: f64,
    steps: usize,
}

/// 累加单次 Agent invocation 内所有 reasoning step 的 token 用量。
///
/// 实例与缓存的 ReActAgent 同生命周期，因此每次调用前必须通过 [`Self::reset`]
/// 清零。AgentScope 当前未完整分发 PostReasoning Hook，Rust 集成同时由
/// `UsageTrackingModel` 在模型流结束时提交同一份逐步 usage。
///
/// 对应 Java: `com.yomahub.liteflow.agent.hook.ChatUsageTrackingHook`。
pub struct ChatUsageTrackingHook {
    state: Mutex<UsageState>,
}

impl ChatUsageTrackingHook {
    /// 创建空的调用级 usage 累加器。
    ///
    /// 对应 Java: `ChatUsageTrackingHook#ChatUsageTrackingHook`。
    #[must_use]
    pub fn new() -> Self {
        Self {
            state: Mutex::new(UsageState::default()),
        }
    }

    /// 清空上一次 invocation 的累计 token、耗时和 reasoning step 数。
    ///
    /// 对应 Java: `ChatUsageTrackingHook#reset`。
    pub fn reset(&self) {
        *self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = UsageState::default();
    }

    /// 返回当前 invocation 的累计 usage 快照。
    ///
    /// # 返回
    /// 至少观察到一个带 usage 的 reasoning step 时返回累计值，否则返回 `None`。
    ///
    /// 对应 Java: `ChatUsageTrackingHook#snapshot`。
    #[must_use]
    pub fn snapshot(&self) -> Option<ChatUsage> {
        let state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        (state.steps > 0).then(|| ChatUsage {
            input_tokens: state.input_tokens,
            output_tokens: state.output_tokens,
            cached_tokens: state.cached_tokens,
            time: state.time,
        })
    }

    /// 返回已累计 usage 的 reasoning step 数。
    ///
    /// 对应 Java: `ChatUsageTrackingHook#getSteps`。
    #[must_use]
    pub fn get_steps(&self) -> usize {
        self.state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .steps
    }

    pub(crate) fn add_usage(&self, usage: &ChatUsage) {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        state.input_tokens = state.input_tokens.saturating_add(usage.input_tokens);
        state.output_tokens = state.output_tokens.saturating_add(usage.output_tokens);
        state.cached_tokens = state.cached_tokens.saturating_add(usage.cached_tokens);
        state.time += usage.time;
        state.steps += 1;
    }
}

impl Default for ChatUsageTrackingHook {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Hook for ChatUsageTrackingHook {
    async fn on_event(&self, event: HookEvent) -> Result<HookEvent, AgentScopeError> {
        if let HookEvent::PostReasoning(data) = &event {
            let _ = data
                .reasoning_message
                .as_ref()
                .and_then(agentscope_core::Msg::get_chat_usage)
                .map(|usage| self.add_usage(&usage));
        }
        Ok(event)
    }
}

#[cfg(test)]
mod tests {
    use agentscope_core::message::ChatUsage;

    use super::ChatUsageTrackingHook;

    #[test]
    fn snapshot_accumulates_steps_and_reset_clears_state() {
        let hook = ChatUsageTrackingHook::new();
        assert!(hook.snapshot().is_none());

        hook.add_usage(&ChatUsage {
            input_tokens: 10,
            output_tokens: 2,
            cached_tokens: 3,
            time: 0.1,
        });
        hook.add_usage(&ChatUsage {
            input_tokens: 20,
            output_tokens: 4,
            cached_tokens: 1,
            time: 0.2,
        });

        let snapshot = hook.snapshot().expect("应存在累计 usage");
        assert_eq!(snapshot.input_tokens, 30);
        assert_eq!(snapshot.output_tokens, 6);
        assert_eq!(snapshot.cached_tokens, 4);
        assert!((snapshot.time - 0.3).abs() < f64::EPSILON);
        assert_eq!(hook.get_steps(), 2);

        hook.reset();
        assert!(hook.snapshot().is_none());
        assert_eq!(hook.get_steps(), 0);
    }
}
