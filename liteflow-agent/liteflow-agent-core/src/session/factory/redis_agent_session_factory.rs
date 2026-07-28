use std::sync::Arc;
use std::sync::OnceLock;

use agentscope_core::session::Session;
use dashmap::DashMap;

use super::AgentSessionFactory;
use crate::{AgentConfig, AgentConfigException, MemoryStorageMode};

static REDIS_SESSIONS: OnceLock<DashMap<String, Arc<dyn Session>>> = OnceLock::new();

/// Redis Session 工厂。
///
/// Java 从 Spring `ContextAware` 按 beanName 取得 Redisson/Jedis/Lettuce 客户端；
/// Rust 由宿主先构造真实 AgentScope `RedisSession`，再按同一 beanName 注册。
/// 这种映射保留命名 Bean、延迟解析和宿主持有连接资源的语义，同时避免把 JVM
/// 客户端类型伪装成 Rust 客户端。
///
/// 对应 Java: `com.yomahub.liteflow.agent.session.factory.RedisAgentSessionFactory`。
pub struct RedisAgentSessionFactory;

impl RedisAgentSessionFactory {
    fn sessions() -> &'static DashMap<String, Arc<dyn Session>> {
        REDIS_SESSIONS.get_or_init(DashMap::new)
    }

    /// 按 Java Redis 客户端 beanName 注册真实 AgentScope Session。
    ///
    /// # 参数
    /// - `bean_name`: 对应 `RedisMemoryConfig#beanName`。
    /// - `session`: 已由宿主使用真实 Redis 客户端构造的 Session。
    ///
    /// # 返回
    /// 同名后端原先注册的 Session；首次注册时返回 `None`。
    ///
    /// 对应 Java: `ContextAware#registerBean` 与
    /// `RedisAgentSessionFactory#create` 的命名 Bean 装配边界。
    pub fn register_session(
        bean_name: impl Into<String>,
        session: Arc<dyn Session>,
    ) -> Result<Option<Arc<dyn Session>>, AgentConfigException> {
        let bean_name = bean_name.into();
        let bean_name = bean_name.trim();
        if bean_name.is_empty() {
            return Err(AgentConfigException::new(
                "Redis Session bean name cannot be blank",
            ));
        }
        Ok(Self::sessions().insert(bean_name.to_string(), session))
    }

    /// 移除指定名称的 Redis Session。
    ///
    /// # 参数
    /// - `bean_name`: 需要移除的配置 beanName。
    ///
    /// # 返回
    /// 被移除的 Session；不存在时返回 `None`。
    pub fn unregister_session(bean_name: &str) -> Option<Arc<dyn Session>> {
        Self::sessions()
            .remove(bean_name.trim())
            .map(|(_, session)| session)
    }
}

impl AgentSessionFactory for RedisAgentSessionFactory {
    fn mode(&self) -> MemoryStorageMode {
        MemoryStorageMode::Redis
    }

    fn create(
        &self,
        agent_config: &AgentConfig,
    ) -> Result<Option<Arc<dyn Session>>, AgentConfigException> {
        let redis = &agent_config.session.memory.redis;
        let bean_name = redis
            .bean_name()
            .map(str::trim)
            .filter(|bean_name| !bean_name.is_empty())
            .ok_or_else(|| {
                AgentConfigException::new(
                    "liteflow.agent.session.memory.redis.beanName is required when mode=REDIS",
                )
            })?;

        // 与 Java 首次 process 时从 ContextAware 解析 Bean 一致：只在工厂真正创建
        // Session 时查询注册表，不把远端连接耦合到 LiteFlow 启动阶段。
        Self::sessions()
            .get(bean_name)
            .map(|session| Some(session.value().clone()))
            .ok_or_else(|| {
                AgentConfigException::new(format!("Redis Session bean not found: {bean_name}"))
            })
    }
}
