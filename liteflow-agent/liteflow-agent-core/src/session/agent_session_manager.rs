//! 对应 Java: `AgentSessionManager`。

use std::sync::Arc;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use dashmap::DashMap;

use crate::{AgentDefinition, session::AgentSessionEntry};

/// 按 conversationId 和 agentKey 缓存 AgentScope ReActAgent。
pub struct AgentSessionManager {
    definition: Arc<AgentDefinition>,
    sessions: DashMap<String, Arc<AgentSessionEntry>>,
    last_cleanup: Mutex<Instant>,
}

impl AgentSessionManager {
    /// 创建会话管理器。
    #[must_use]
    pub fn new(definition: Arc<AgentDefinition>) -> Self {
        Self {
            definition,
            sessions: DashMap::new(),
            last_cleanup: Mutex::new(Instant::now()),
        }
    }

    /// 获取或创建一个会话缓存项。
    pub(crate) fn get_or_create(&self, conversation_id: &str) -> Arc<AgentSessionEntry> {
        self.cleanup_if_due();
        let key = format!("{}:{conversation_id}", self.definition.agent_key());
        let session = self
            .sessions
            .entry(key)
            .or_insert_with(|| {
                Arc::new(AgentSessionEntry::new(
                    self.definition.build_agent(conversation_id),
                ))
            })
            .clone();
        session.touch();
        self.enforce_max_sessions();
        session
    }

    /// 返回已缓存会话数。
    #[must_use]
    pub fn session_count(&self) -> usize {
        self.sessions.len()
    }

    /// 删除指定会话。
    pub fn remove(&self, conversation_id: &str) -> bool {
        let key = format!("{}:{conversation_id}", self.definition.agent_key());
        self.sessions.remove(&key).is_some()
    }

    /// 清空 Agent 实例缓存；持久 Session 数据由具体 AgentScope Session 管理。
    pub fn clear(&self) {
        self.sessions.clear();
    }

    fn cleanup_if_due(&self) {
        let cleanup_interval = self
            .definition
            .config()
            .session
            .cleanup_interval
            .max(Duration::from_millis(20));
        let mut last_cleanup = self
            .last_cleanup
            .lock()
            .expect("agent session cleanup lock poisoned");
        if last_cleanup.elapsed() < cleanup_interval {
            return;
        }
        *last_cleanup = Instant::now();
        let idle_timeout = self.definition.config().session.idle_timeout;

        // Rust 无常驻 JVM 清理线程；在会话获取入口按配置周期执行等价的惰性清理，
        // 不为每个组件额外占用一个后台 Tokio 任务。
        self.sessions
            .retain(|_, session| session.last_active().elapsed() <= idle_timeout);
    }

    fn enforce_max_sessions(&self) {
        let max_sessions = self.definition.config().session.max_sessions;
        while self.sessions.len() > max_sessions {
            let oldest_key = self
                .sessions
                .iter()
                .min_by_key(|entry| entry.value().last_active())
                .map(|entry| entry.key().clone());
            if let Some(oldest_key) = oldest_key {
                self.sessions.remove(&oldest_key);
            } else {
                break;
            }
        }
    }
}
