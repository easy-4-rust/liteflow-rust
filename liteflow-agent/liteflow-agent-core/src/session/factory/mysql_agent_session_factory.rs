use std::sync::Arc;
use std::sync::OnceLock;

use agentscope_core::session::Session;
use dashmap::DashMap;

use super::AgentSessionFactory;
use crate::{AgentConfig, AgentConfigException, MemoryStorageMode};

static MYSQL_SESSIONS: OnceLock<DashMap<String, Arc<dyn Session>>> = OnceLock::new();

/// MySQL Session 工厂。
///
/// Java 从 Spring `ContextAware` 按 dataSourceBeanName 取得 `DataSource`；
/// Rust 由宿主使用连接池构造真实 AgentScope `MysqlSession` 后，以同名资源注册。
/// 数据库连接、密钥和池生命周期仍由宿主负责，LiteFlow 只完成配置到 Session
/// 的命名解析。
///
/// 对应 Java: `com.yomahub.liteflow.agent.session.factory.MysqlAgentSessionFactory`。
pub struct MysqlAgentSessionFactory;

impl MysqlAgentSessionFactory {
    fn sessions() -> &'static DashMap<String, Arc<dyn Session>> {
        MYSQL_SESSIONS.get_or_init(DashMap::new)
    }

    /// 按 Java DataSource beanName 注册真实 AgentScope Session。
    ///
    /// # 参数
    /// - `data_source_bean_name`: 对应 `MysqlMemoryConfig#dataSourceBeanName`。
    /// - `session`: 已由宿主使用真实 MySQL 连接池构造的 Session。
    ///
    /// # 返回
    /// 同名后端原先注册的 Session；首次注册时返回 `None`。
    ///
    /// 对应 Java: `ContextAware#registerBean` 与
    /// `MysqlAgentSessionFactory#create` 的命名 DataSource 装配边界。
    pub fn register_session(
        data_source_bean_name: impl Into<String>,
        session: Arc<dyn Session>,
    ) -> Result<Option<Arc<dyn Session>>, AgentConfigException> {
        let data_source_bean_name = data_source_bean_name.into();
        let data_source_bean_name = data_source_bean_name.trim();
        if data_source_bean_name.is_empty() {
            return Err(AgentConfigException::new(
                "MySQL Session data source bean name cannot be blank",
            ));
        }
        Ok(Self::sessions().insert(data_source_bean_name.to_string(), session))
    }

    /// 移除指定名称的 MySQL Session。
    ///
    /// # 参数
    /// - `data_source_bean_name`: 需要移除的配置 DataSource 名称。
    ///
    /// # 返回
    /// 被移除的 Session；不存在时返回 `None`。
    pub fn unregister_session(data_source_bean_name: &str) -> Option<Arc<dyn Session>> {
        Self::sessions()
            .remove(data_source_bean_name.trim())
            .map(|(_, session)| session)
    }
}

impl AgentSessionFactory for MysqlAgentSessionFactory {
    fn mode(&self) -> MemoryStorageMode {
        MemoryStorageMode::Mysql
    }

    fn create(
        &self,
        agent_config: &AgentConfig,
    ) -> Result<Option<Arc<dyn Session>>, AgentConfigException> {
        let mysql = &agent_config.session.memory.mysql;
        let data_source_bean_name = mysql
            .data_source_bean_name()
            .map(str::trim)
            .filter(|bean_name| !bean_name.is_empty())
            .ok_or_else(|| {
                AgentConfigException::new(
                    "liteflow.agent.session.memory.mysql.dataSourceBeanName is required when mode=MYSQL",
                )
            })?;

        // Java 的数据库名、表名和自动建表选项在 DataSource 创建 MysqlSession 时
        // 生效；Rust 对等地要求宿主构造 Session 时应用这些设置，工厂负责命名解析。
        Self::sessions()
            .get(data_source_bean_name)
            .map(|session| Some(session.value().clone()))
            .ok_or_else(|| {
                AgentConfigException::new(format!(
                    "MySQL Session data source bean not found: {data_source_bean_name}"
                ))
            })
    }
}
