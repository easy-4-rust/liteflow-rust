//! 对应 Java: `AgentSessionManager`。

use std::sync::Arc;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use dashmap::DashMap;
use dashmap::mapref::entry::Entry;

use crate::{
    AgentDefinition, AgentError,
    session::{AgentSession, workspace_lifecycle_coordinator::workspace_lifecycle_coordinator},
};

/// 按 conversationId 和 agentKey 缓存 AgentScope ReActAgent。
pub struct AgentSessionManager {
    definition: Arc<AgentDefinition>,
    sessions: DashMap<String, Arc<AgentSession>>,
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
    pub(crate) async fn get_or_create(
        &self,
        conversation_id: &str,
    ) -> Result<Arc<AgentSession>, AgentError> {
        self.cleanup_if_due();
        let key = cache_key(conversation_id, self.definition.agent_key());
        if let Some(session) = self.sessions.get(&key) {
            let session = Arc::clone(session.value());
            session.touch();
            return Ok(session);
        }
        let (
            agent,
            skill_tracking_hook,
            react_logging_hook,
            chat_usage_tracking_hook,
            workspace_dir,
        ) = self.definition.build_agent(conversation_id).await?;
        let candidate = Arc::new(AgentSession::new(
            safe_id(conversation_id),
            safe_id(self.definition.agent_key()),
            key.clone(),
            agent,
            skill_tracking_hook,
            react_logging_hook,
            chat_usage_tracking_hook,
            workspace_dir,
        ));
        if let Some(workspace_dir) = candidate.workspace_dir() {
            workspace_lifecycle_coordinator().acquire(workspace_dir);
        }
        let session = match self.sessions.entry(key) {
            Entry::Occupied(entry) => {
                if let Some(workspace_dir) = candidate.workspace_dir() {
                    workspace_lifecycle_coordinator().release(workspace_dir, false);
                }
                Arc::clone(entry.get())
            }
            Entry::Vacant(entry) => Arc::clone(&entry.insert(candidate)),
        };
        session.touch();
        self.enforce_max_sessions();
        Ok(session)
    }

    /// 返回已缓存会话数。
    #[must_use]
    pub fn session_count(&self) -> usize {
        self.sessions.len()
    }

    /// 判断指定 conversation 与当前 Agent key 的会话是否已缓存。
    ///
    /// 对应 Java: `AgentSessionManager#contains`。
    #[must_use]
    pub fn contains(&self, conversation_id: &str) -> bool {
        let key = cache_key(conversation_id, self.definition.agent_key());
        self.sessions.contains_key(&key)
    }

    /// 返回指定 conversation 的已缓存会话，不触发新建。
    ///
    /// # 参数
    /// - `conversation_id`: 原始业务对话标识，内部按 Java `safeId` 规则编码。
    ///
    /// # 返回
    /// 会话存在时返回共享句柄，否则返回 `None`。
    #[must_use]
    pub fn session(&self, conversation_id: &str) -> Option<Arc<AgentSession>> {
        let key = cache_key(conversation_id, self.definition.agent_key());
        self.sessions
            .get(&key)
            .map(|session| Arc::clone(session.value()))
    }

    /// 返回指定会话在最近一次 invocation 中成功加载的技能名称。
    ///
    /// # 参数
    /// - `conversation_id`: 业务会话标识。
    ///
    /// # 返回
    /// 按首次加载顺序去重的技能名称；会话不存在或未启用 Skills 时返回空列表。
    ///
    /// 对应 Java: `ReActAgentComponent#usedSkills`。
    #[must_use]
    pub fn used_skills(&self, conversation_id: &str) -> Vec<String> {
        let key = cache_key(conversation_id, self.definition.agent_key());
        self.sessions
            .get(&key)
            .map_or_else(Vec::new, |session| session.used_skills())
    }

    /// 返回指定会话最近一次 invocation 的累计模型 usage。
    ///
    /// 对应 Java: `ReActAgentContext#getChatUsage`。
    #[must_use]
    pub fn chat_usage(&self, conversation_id: &str) -> Option<agentscope_core::message::ChatUsage> {
        let key = cache_key(conversation_id, self.definition.agent_key());
        self.sessions
            .get(&key)
            .and_then(|session| session.chat_usage())
    }

    /// 返回指定会话最近一次 invocation 的 reasoning usage step 数。
    ///
    /// 对应 Java: `ChatUsageTrackingHook#getSteps`。
    #[must_use]
    pub fn chat_usage_steps(&self, conversation_id: &str) -> usize {
        let key = cache_key(conversation_id, self.definition.agent_key());
        self.sessions
            .get(&key)
            .map_or(0, |session| session.chat_usage_steps())
    }

    /// 删除指定会话。
    pub fn remove(&self, conversation_id: &str) -> bool {
        let key = cache_key(conversation_id, self.definition.agent_key());
        self.sessions.remove(&key).is_some_and(|(_, session)| {
            self.release_workspace(&session, false);
            true
        })
    }

    /// 清空 Agent 实例缓存；持久 Session 数据由具体 AgentScope Session 管理。
    pub fn clear(&self) {
        let sessions = self
            .sessions
            .iter()
            .map(|entry| Arc::clone(entry.value()))
            .collect::<Vec<_>>();
        for session in sessions {
            if self.sessions.remove(session.cache_key()).is_some() {
                self.release_workspace(&session, false);
            }
        }
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

        // Rust 无常驻 JVM 清理线程；在会话获取入口按配置周期执行等价的惰性清理。
        // 正在执行的会话无法取得 gate，因此与 Java tryLock 一样跳过本轮。
        let victims = self
            .sessions
            .iter()
            .filter(|entry| entry.value().last_active().elapsed() > idle_timeout)
            .map(|entry| Arc::clone(entry.value()))
            .collect::<Vec<_>>();
        for victim in victims {
            let Ok(_gate) = victim.gate().try_lock() else {
                continue;
            };
            self.evict_from_cache(
                &victim,
                self.definition.config().workspace.cleanup_on_session_expire,
            );
        }
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
                // LRU 只淘汰进程内 Agent；与 Java 一样保留工作区和持久 Session。
                if let Some((_, session)) = self.sessions.remove(&oldest_key) {
                    self.release_workspace(&session, false);
                }
            } else {
                break;
            }
        }
    }

    fn evict_from_cache(&self, session: &AgentSession, clean_workspace: bool) {
        if let Some((_, removed)) = self.sessions.remove(session.cache_key()) {
            self.release_workspace(&removed, clean_workspace);
        }
    }

    fn release_workspace(&self, session: &AgentSession, clean_workspace: bool) {
        if let Some(workspace_dir) = session.workspace_dir() {
            workspace_lifecycle_coordinator().release(workspace_dir, clean_workspace);
        }
    }
}

impl Drop for AgentSessionManager {
    fn drop(&mut self) {
        let clean_workspace = self.definition.config().workspace.cleanup_on_jvm_shutdown;
        let sessions = self
            .sessions
            .iter()
            .map(|entry| Arc::clone(entry.value()))
            .collect::<Vec<_>>();
        for session in sessions {
            if self.sessions.remove(session.cache_key()).is_some() {
                self.release_workspace(&session, clean_workspace);
            }
        }
    }
}

fn cache_key(conversation_id: &str, agent_key: &str) -> String {
    format!("{}__{}", safe_id(conversation_id), safe_id(agent_key))
}

fn safe_id(raw: &str) -> String {
    if raw.is_empty() {
        return "_".to_string();
    }
    if raw
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
    {
        return raw.to_string();
    }
    let mut encoded = String::new();
    for byte in raw.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_' | b'*') {
            encoded.push(char::from(byte));
        } else if byte == b' ' {
            encoded.push('+');
        } else {
            encoded.push('_');
            encoded.push_str(&format!("{byte:02X}"));
        }
    }
    encoded
}
