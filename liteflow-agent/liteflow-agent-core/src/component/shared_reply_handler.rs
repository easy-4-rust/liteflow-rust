use std::sync::Arc;

use agentscope_core::Msg;
use liteflow_core::LFResult;

use super::ReActAgentContext;

/// `ReActAgentComponent#handleReply` 的线程安全 Rust 回调类型。
pub(crate) type SharedReplyHandler =
    Arc<dyn Fn(&ReActAgentContext, &Msg) -> LFResult<()> + Send + Sync>;
