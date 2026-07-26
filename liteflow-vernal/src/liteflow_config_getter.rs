//! LiteFlow 兼容配置获取器。

use std::sync::{OnceLock, RwLock};

use crate::LiteflowConfig;

fn config_cell() -> &'static RwLock<Option<LiteflowConfig>> {
    static CONFIG: OnceLock<RwLock<Option<LiteflowConfig>>> = OnceLock::new();
    CONFIG.get_or_init(|| RwLock::new(None))
}

/// LiteFlow 配置的进程级兼容访问入口。
///
/// 对应 Java: `com.yomahub.liteflow.property.LiteflowConfigGetter`。
/// Vernal 内部仍优先通过容器注入不可变配置；此对象服务于不在 IoC 管理范围内、
/// 但需要读取当前配置的兼容扩展点。尚未设置时返回默认配置，与 Java 回退
/// `new LiteflowConfig()` 的行为一致。
pub struct LiteflowConfigGetter;

impl LiteflowConfigGetter {
    /// 获取当前配置的快照。
    ///
    /// 对应 Java: `LiteflowConfigGetter#get`。返回克隆值，避免调用方绕过 Vernal
    /// 生命周期并发修改运行中的配置。
    #[must_use]
    pub fn get() -> LiteflowConfig {
        config_cell().read().unwrap().clone().unwrap_or_default()
    }

    /// 设置当前兼容配置。
    ///
    /// 对应 Java: `LiteflowConfigGetter#setLiteflowConfig`。Vernal 模块完成配置
    /// 装配时调用此方法。
    pub fn set_liteflow_config(config: LiteflowConfig) {
        *config_cell().write().unwrap() = Some(config);
    }

    /// 清空当前兼容配置。
    ///
    /// 对应 Java: `LiteflowConfigGetter#clean`。后续 `get` 将重新返回默认配置。
    pub fn clean() {
        *config_cell().write().unwrap() = None;
    }
}
