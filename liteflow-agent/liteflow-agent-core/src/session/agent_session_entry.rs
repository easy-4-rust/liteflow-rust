//! 单个 `(conversationId, agentKey)` 会话缓存项。

use std::sync::Arc;
use std::sync::Mutex as StdMutex;
use std::time::Instant;

use agentscope_core::ReActAgent;
use tokio::sync::Mutex;

/// 缓存 Agent，并串行化同一会话上的调用。
pub(crate) struct AgentSessionEntry {
    agent: Arc<ReActAgent>,
    gate: Mutex<()>,
    last_active: StdMutex<Instant>,
}

impl AgentSessionEntry {
    /// 创建缓存项。
    pub(crate) fn new(agent: ReActAgent) -> Self {
        Self {
            agent: Arc::new(agent),
            gate: Mutex::new(()),
            last_active: StdMutex::new(Instant::now()),
        }
    }

    /// 返回 Agent。
    pub(crate) fn agent(&self) -> &Arc<ReActAgent> {
        &self.agent
    }

    /// 返回并发闸门。
    pub(crate) fn gate(&self) -> &Mutex<()> {
        &self.gate
    }

    /// 刷新最近访问时间。对应 Java: `AgentSession#touch`。
    pub(crate) fn touch(&self) {
        *self
            .last_active
            .lock()
            .expect("agent session last_active lock poisoned") = Instant::now();
    }

    /// 返回最近访问时间。对应 Java: `AgentSession#getLastActive`。
    pub(crate) fn last_active(&self) -> Instant {
        *self
            .last_active
            .lock()
            .expect("agent session last_active lock poisoned")
    }
}
