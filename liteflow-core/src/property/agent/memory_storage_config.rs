use serde::{Deserialize, Serialize};

use super::{LocalFileMemoryConfig, MemoryStorageMode, MysqlMemoryConfig, RedisMemoryConfig};

/// ReAct Agent 会话记忆持久化设置。
///
/// 本配置决定对话历史持久化到进程内、本地文件、Redis 或 MySQL；它与控制
/// Agent 实例缓存时长的 `SessionConfig` 相互独立。
///
/// 对应 Java: `com.yomahub.liteflow.property.agent.MemoryStorageConfig`。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct MemoryStorageConfig {
    /// 记忆存储后端，默认与 Java 一致为 JVM。
    pub mode: MemoryStorageMode,
    /// LOCAL_FILE 模式子配置。
    pub local_file: LocalFileMemoryConfig,
    /// REDIS 模式子配置。
    pub redis: RedisMemoryConfig,
    /// MYSQL 模式子配置。
    pub mysql: MysqlMemoryConfig,
    /// 是否在首次使用时加载已有会话。
    pub load_on_first_use: bool,
    /// 成功调用后是否保存状态。
    pub save_after_call: bool,
    /// 调用失败时是否仍保存状态。
    pub save_on_error: bool,
}

impl Default for MemoryStorageConfig {
    fn default() -> Self {
        Self {
            mode: MemoryStorageMode::Jvm,
            local_file: LocalFileMemoryConfig,
            redis: RedisMemoryConfig::default(),
            mysql: MysqlMemoryConfig::default(),
            load_on_first_use: true,
            save_after_call: true,
            save_on_error: true,
        }
    }
}

impl MemoryStorageConfig {
    /// 返回记忆后端。对应 Java: `MemoryStorageConfig#getMode`。
    #[must_use]
    pub fn mode(&self) -> MemoryStorageMode {
        self.mode
    }

    /// 返回记忆存储后端。
    ///
    /// # 返回
    /// 当前配置使用的 JVM、文件、Redis、MySQL 或禁用模式。
    ///
    /// 对应 Java: `MemoryStorageConfig#getMode`。
    #[must_use]
    pub fn get_mode(&self) -> MemoryStorageMode {
        self.mode()
    }

    /// 设置记忆后端。对应 Java: `MemoryStorageConfig#setMode`。
    pub fn set_mode(&mut self, mode: MemoryStorageMode) {
        self.mode = mode;
    }

    /// 返回本地文件配置。对应 Java: `MemoryStorageConfig#getLocalFile`。
    #[must_use]
    pub fn local_file(&self) -> &LocalFileMemoryConfig {
        &self.local_file
    }

    /// 返回本地文件记忆子配置。
    ///
    /// # 返回
    /// 与 serde 反序列化及 Agent 会话工厂共享的真实配置对象。
    ///
    /// 对应 Java: `MemoryStorageConfig#getLocalFile`。
    #[must_use]
    pub fn get_local_file(&self) -> &LocalFileMemoryConfig {
        self.local_file()
    }

    /// 设置本地文件配置。对应 Java: `MemoryStorageConfig#setLocalFile`。
    pub fn set_local_file(&mut self, local_file: LocalFileMemoryConfig) {
        self.local_file = local_file;
    }

    /// 返回 Redis 配置。对应 Java: `MemoryStorageConfig#getRedis`。
    #[must_use]
    pub fn redis(&self) -> &RedisMemoryConfig {
        &self.redis
    }

    /// 返回 Redis 记忆子配置。
    ///
    /// # 返回
    /// 与 Agent 会话工厂共享的真实 Redis 配置对象。
    ///
    /// 对应 Java: `MemoryStorageConfig#getRedis`。
    #[must_use]
    pub fn get_redis(&self) -> &RedisMemoryConfig {
        self.redis()
    }

    /// 设置 Redis 配置。对应 Java: `MemoryStorageConfig#setRedis`。
    pub fn set_redis(&mut self, redis: RedisMemoryConfig) {
        self.redis = redis;
    }

    /// 返回 MySQL 配置。对应 Java: `MemoryStorageConfig#getMysql`。
    #[must_use]
    pub fn mysql(&self) -> &MysqlMemoryConfig {
        &self.mysql
    }

    /// 返回 MySQL 记忆子配置。
    ///
    /// # 返回
    /// 与 Agent 会话工厂共享的真实 MySQL 配置对象。
    ///
    /// 对应 Java: `MemoryStorageConfig#getMysql`。
    #[must_use]
    pub fn get_mysql(&self) -> &MysqlMemoryConfig {
        self.mysql()
    }

    /// 设置 MySQL 配置。对应 Java: `MemoryStorageConfig#setMysql`。
    pub fn set_mysql(&mut self, mysql: MysqlMemoryConfig) {
        self.mysql = mysql;
    }

    /// 返回是否首次使用时加载。对应 Java: `MemoryStorageConfig#isLoadOnFirstUse`。
    #[must_use]
    pub fn is_load_on_first_use(&self) -> bool {
        self.load_on_first_use
    }

    /// 设置首次使用加载开关。对应 Java: `MemoryStorageConfig#setLoadOnFirstUse`。
    pub fn set_load_on_first_use(&mut self, load_on_first_use: bool) {
        self.load_on_first_use = load_on_first_use;
    }

    /// 返回成功调用后是否保存。对应 Java: `MemoryStorageConfig#isSaveAfterCall`。
    #[must_use]
    pub fn is_save_after_call(&self) -> bool {
        self.save_after_call
    }

    /// 设置成功调用后的保存开关。对应 Java: `MemoryStorageConfig#setSaveAfterCall`。
    pub fn set_save_after_call(&mut self, save_after_call: bool) {
        self.save_after_call = save_after_call;
    }

    /// 返回失败时是否保存。对应 Java: `MemoryStorageConfig#isSaveOnError`。
    #[must_use]
    pub fn is_save_on_error(&self) -> bool {
        self.save_on_error
    }

    /// 设置失败时保存开关。对应 Java: `MemoryStorageConfig#setSaveOnError`。
    pub fn set_save_on_error(&mut self, save_on_error: bool) {
        self.save_on_error = save_on_error;
    }
}
