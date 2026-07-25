use std::sync::Arc;

use crate::spi::{SpiFactory, liteflow_component_support::LiteflowComponentSupport};

/// LiteFlow 组件支持持有者
///
/// 用于获取 LiteFlow 组件支持实例
pub struct LiteflowComponentSupportHolder {
    /// LiteFlow 组件支持实例
    liteflow_component_support: Arc<dyn LiteflowComponentSupport>,
}

impl LiteflowComponentSupportHolder {
    /// 创建 LiteFlow 组件支持持有者
    ///
    /// 通过 SPI 工厂获取 LiteFlow 组件支持实例
    pub fn new() -> Self {
        let liteflow_component_support = SpiFactory::liteflow_component_support();
        Self {
            liteflow_component_support,
        }
    }

    /// 获取 LiteFlow 组件支持实例
    ///
    /// 返回 LiteFlow 组件支持实例
    pub fn liteflow_component_support(&self) -> Arc<dyn LiteflowComponentSupport> {
        self.liteflow_component_support.clone()
    }
}
