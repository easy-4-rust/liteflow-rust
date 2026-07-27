//! LiteFlow 全局配置获取器。

use std::sync::{OnceLock, RwLock};

use super::LiteflowConfig;

fn config_cell() -> &'static RwLock<Option<LiteflowConfig>> {
    static CONFIG: OnceLock<RwLock<Option<LiteflowConfig>>> = OnceLock::new();
    CONFIG.get_or_init(|| RwLock::new(None))
}

/// 为核心引擎和扩展点提供进程级 LiteFlow 配置。
///
/// Java 会先尝试从 `ContextAwareHolder` 获取容器 Bean；Rust 的容器装配层会在
/// 创建 `FlowExecutor` 时显式写入配置。独立运行且尚未装配时回退到
/// `LiteflowConfig::default()`。
///
/// 对应 Java: `com.yomahub.liteflow.property.LiteflowConfigGetter`。
pub struct LiteflowConfigGetter;

impl LiteflowConfigGetter {
    /// 获取当前配置快照；未设置时返回默认配置。
    ///
    /// 返回克隆值，避免调用方绕过执行器装配过程并发修改全局配置。
    /// 对应 Java: `LiteflowConfigGetter#get`。
    #[must_use]
    pub fn get() -> LiteflowConfig {
        config_cell().read().unwrap().clone().unwrap_or_default()
    }

    /// 设置当前全局配置。
    ///
    /// 参数 `liteflow_config` 对应 Java 同名参数。
    /// 对应 Java: `LiteflowConfigGetter#setLiteflowConfig`。
    pub fn set_liteflow_config(liteflow_config: LiteflowConfig) {
        *config_cell().write().unwrap() = Some(liteflow_config);
    }

    /// 清空当前配置，后续读取重新回退到默认值。
    ///
    /// 对应 Java: `LiteflowConfigGetter#clean`。
    pub fn clean() {
        *config_cell().write().unwrap() = None;
    }
}

#[cfg(test)]
mod tests {
    use super::LiteflowConfigGetter;
    use crate::property::LiteflowConfig;

    #[test]
    fn set_get_and_clean_preserve_java_fallback_contract() {
        LiteflowConfigGetter::clean();
        assert_eq!(LiteflowConfigGetter::get(), LiteflowConfig::default());

        let mut configured = LiteflowConfig::default();
        configured.set_slot_size(73);
        LiteflowConfigGetter::set_liteflow_config(configured.clone());
        assert_eq!(LiteflowConfigGetter::get(), configured);

        LiteflowConfigGetter::clean();
        assert_eq!(LiteflowConfigGetter::get(), LiteflowConfig::default());
    }
}
