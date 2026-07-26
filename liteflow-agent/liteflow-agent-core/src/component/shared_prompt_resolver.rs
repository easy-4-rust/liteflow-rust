//! ReAct Agent 用户提示词解析器的共享类型。

use std::sync::Arc;

use liteflow_core::{CmpContext, LFResult};

/// 根据当前 LiteFlow 组件上下文解析用户提示词。
///
/// 对应 Java: `ReActAgentComponentBuilder` 中传入组件执行上下文的提示词解析回调。
pub(crate) type SharedPromptResolver =
    Arc<dyn Fn(&CmpContext) -> LFResult<String> + Send + Sync + 'static>;
