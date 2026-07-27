use serde::{Deserialize, Serialize};

/// Agent 工作区配置，对应配置段 `liteflow.agent.workspace.*`。
///
/// 每个会话拥有独立本地目录，生命周期由会话管理器控制，文件工具按大小和列表
/// 数量限制访问，防止模型读写超大文件或返回过多目录项。
///
/// 对应 Java: `com.yomahub.liteflow.property.agent.WorkspaceConfig`。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct WorkspaceConfig {
    /// 工作区根目录；未配置时跳过工作区相关初始化。
    pub root: Option<String>,
    /// 是否自动创建根目录。
    pub auto_create: bool,
    /// 会话超时淘汰时是否清理工作区。
    pub cleanup_on_session_expire: bool,
    /// 进程关闭时是否清理工作区。
    pub cleanup_on_jvm_shutdown: bool,
    /// 单个文件可读写的最大字节数。
    pub max_file_bytes: u64,
    /// 列目录一次最多返回的条目数。
    pub max_list_size: usize,
}

impl Default for WorkspaceConfig {
    fn default() -> Self {
        Self {
            root: None,
            auto_create: true,
            cleanup_on_session_expire: true,
            cleanup_on_jvm_shutdown: false,
            max_file_bytes: 10 * 1024 * 1024,
            max_list_size: 1_000,
        }
    }
}

impl WorkspaceConfig {
    /// 返回工作区根目录。对应 Java: `WorkspaceConfig#getRoot`。
    #[must_use]
    pub fn root(&self) -> Option<&str> {
        self.root.as_deref()
    }

    /// 返回工作区根目录。
    ///
    /// # 返回
    /// 未配置工作区时返回 `None`，否则返回会话管理器实际使用的根目录。
    ///
    /// 对应 Java: `WorkspaceConfig#getRoot`。
    #[must_use]
    pub fn get_root(&self) -> Option<&str> {
        self.root()
    }

    /// 设置工作区根目录。对应 Java: `WorkspaceConfig#setRoot`。
    pub fn set_root(&mut self, root: Option<String>) {
        self.root = root;
    }

    /// 返回是否自动创建根目录。对应 Java: `WorkspaceConfig#isAutoCreate`。
    #[must_use]
    pub fn is_auto_create(&self) -> bool {
        self.auto_create
    }

    /// 设置自动创建开关。对应 Java: `WorkspaceConfig#setAutoCreate`。
    pub fn set_auto_create(&mut self, auto_create: bool) {
        self.auto_create = auto_create;
    }

    /// 返回会话淘汰时是否清理目录。对应 Java: `WorkspaceConfig#isCleanupOnSessionExpire`。
    #[must_use]
    pub fn is_cleanup_on_session_expire(&self) -> bool {
        self.cleanup_on_session_expire
    }

    /// 设置会话淘汰清理开关。对应 Java: `WorkspaceConfig#setCleanupOnSessionExpire`。
    pub fn set_cleanup_on_session_expire(&mut self, cleanup_on_session_expire: bool) {
        self.cleanup_on_session_expire = cleanup_on_session_expire;
    }

    /// 返回进程关闭时是否清理目录。对应 Java: `WorkspaceConfig#isCleanupOnJvmShutdown`。
    #[must_use]
    pub fn is_cleanup_on_jvm_shutdown(&self) -> bool {
        self.cleanup_on_jvm_shutdown
    }

    /// 设置进程关闭清理开关。对应 Java: `WorkspaceConfig#setCleanupOnJvmShutdown`。
    pub fn set_cleanup_on_jvm_shutdown(&mut self, cleanup_on_jvm_shutdown: bool) {
        self.cleanup_on_jvm_shutdown = cleanup_on_jvm_shutdown;
    }

    /// 返回单文件大小上限。对应 Java: `WorkspaceConfig#getMaxFileBytes`。
    #[must_use]
    pub fn max_file_bytes(&self) -> u64 {
        self.max_file_bytes
    }

    /// 返回单个工作区文件允许读写的最大字节数。
    ///
    /// 对应 Java: `WorkspaceConfig#getMaxFileBytes`。
    #[must_use]
    pub fn get_max_file_bytes(&self) -> u64 {
        self.max_file_bytes()
    }

    /// 设置单文件大小上限。对应 Java: `WorkspaceConfig#setMaxFileBytes`。
    pub fn set_max_file_bytes(&mut self, max_file_bytes: u64) {
        self.max_file_bytes = max_file_bytes;
    }

    /// 返回目录列表条目上限。对应 Java: `WorkspaceConfig#getMaxListSize`。
    #[must_use]
    pub fn max_list_size(&self) -> usize {
        self.max_list_size
    }

    /// 返回一次目录列表允许返回的最大条目数。
    ///
    /// 对应 Java: `WorkspaceConfig#getMaxListSize`。
    #[must_use]
    pub fn get_max_list_size(&self) -> usize {
        self.max_list_size()
    }

    /// 设置目录列表条目上限。对应 Java: `WorkspaceConfig#setMaxListSize`。
    pub fn set_max_list_size(&mut self, max_list_size: usize) {
        self.max_list_size = max_list_size;
    }
}
