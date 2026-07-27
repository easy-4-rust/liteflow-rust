use std::time::Duration;

use serde::{Deserialize, Serialize};

use super::MemoryStorageConfig;

/// Agent 会话生命周期配置。
///
/// 控制进程内 Agent 实例的空闲超时、清理周期、数量上限；记忆持久化位置由
/// 独立的 `MemoryStorageConfig` 决定。
///
/// 对应 Java: `com.yomahub.liteflow.property.agent.SessionConfig`。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct SessionConfig {
    /// 会话空闲超时时间，默认 30 分钟。
    #[serde(with = "humantime_serde")]
    pub idle_timeout: Duration,
    /// 后台清理周期，默认 1 分钟。
    #[serde(with = "humantime_serde")]
    pub cleanup_interval: Duration,
    /// 同时存活的 Agent 会话数量上限。
    pub max_sessions: usize,
    /// 会话记忆持久化配置。
    pub memory: MemoryStorageConfig,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            idle_timeout: Duration::from_secs(30 * 60),
            cleanup_interval: Duration::from_secs(60),
            max_sessions: 10_000,
            memory: MemoryStorageConfig::default(),
        }
    }
}

impl SessionConfig {
    /// 返回会话空闲超时。对应 Java: `SessionConfig#getIdleTimeout`。
    #[must_use]
    pub fn idle_timeout(&self) -> Duration {
        self.idle_timeout
    }

    /// 返回 Agent 实例的空闲超时时间。
    ///
    /// # 返回
    /// 会话管理器淘汰长期未访问实例时使用的时长。
    ///
    /// 对应 Java: `SessionConfig#getIdleTimeout`。
    #[must_use]
    pub fn get_idle_timeout(&self) -> Duration {
        self.idle_timeout()
    }

    /// 设置会话空闲超时。对应 Java: `SessionConfig#setIdleTimeout`。
    pub fn set_idle_timeout(&mut self, idle_timeout: Duration) {
        self.idle_timeout = idle_timeout;
    }

    /// 返回后台清理周期。对应 Java: `SessionConfig#getCleanupInterval`。
    #[must_use]
    pub fn cleanup_interval(&self) -> Duration {
        self.cleanup_interval
    }

    /// 返回会话清理周期。
    ///
    /// # 返回
    /// Rust 惰性清理器检查过期会话时使用的间隔。
    ///
    /// 对应 Java: `SessionConfig#getCleanupInterval`。
    #[must_use]
    pub fn get_cleanup_interval(&self) -> Duration {
        self.cleanup_interval()
    }

    /// 设置后台清理周期。对应 Java: `SessionConfig#setCleanupInterval`。
    pub fn set_cleanup_interval(&mut self, cleanup_interval: Duration) {
        self.cleanup_interval = cleanup_interval;
    }

    /// 返回最大会话数。对应 Java: `SessionConfig#getMaxSessions`。
    #[must_use]
    pub fn max_sessions(&self) -> usize {
        self.max_sessions
    }

    /// 返回进程内 Agent 会话数量上限。
    ///
    /// # 返回
    /// LRU 淘汰逻辑使用的最大会话数。
    ///
    /// 对应 Java: `SessionConfig#getMaxSessions`。
    #[must_use]
    pub fn get_max_sessions(&self) -> usize {
        self.max_sessions()
    }

    /// 设置最大会话数。对应 Java: `SessionConfig#setMaxSessions`。
    pub fn set_max_sessions(&mut self, max_sessions: usize) {
        self.max_sessions = max_sessions;
    }

    /// 返回记忆持久化配置。对应 Java: `SessionConfig#getMemory`。
    #[must_use]
    pub fn memory(&self) -> &MemoryStorageConfig {
        &self.memory
    }

    /// 返回会话记忆持久化配置。
    ///
    /// # 返回
    /// 决定记忆加载、保存开关及后端类型的真实配置对象。
    ///
    /// 对应 Java: `SessionConfig#getMemory`。
    #[must_use]
    pub fn get_memory(&self) -> &MemoryStorageConfig {
        self.memory()
    }

    /// 设置记忆持久化配置。对应 Java: `SessionConfig#setMemory`。
    pub fn set_memory(&mut self, memory: MemoryStorageConfig) {
        self.memory = memory;
    }
}
