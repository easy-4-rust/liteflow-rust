//! 对应 Java 类：com.yomahub.liteflow.spi.local.LocalLiteflowComponentSupport
//!
//! 非 spring 环境 LiteflowComponent 注解的处理器。
//! Java 版返回 null（非 Spring 环境不支持 @LiteflowComponent 注解）；
//! Rust 使用 `None` 精确保留该可空语义。

use crate::core::NodeComponent;
use crate::spi::liteflow_component_support::LiteflowComponentSupport;
use crate::spi::spi_priority::SpiPriority;

/// 非容器环境的 LiteFlow 组件名称解析器。
///
/// 对应 Java:
/// `com.yomahub.liteflow.spi.local.LocalLiteflowComponentSupport`。
#[derive(Default)]
pub struct LocalLiteflowComponentSupport;

impl LocalLiteflowComponentSupport {
    /// 创建本地组件名称解析器。
    ///
    /// 对应 Java: `LocalLiteflowComponentSupport#LocalLiteflowComponentSupport`。
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// 返回组件在本地环境中的注解名称。
    ///
    /// 参数 `component` 为已构造的节点组件；非容器环境不处理
    /// `@LiteflowComponent`，因此始终返回 `None`。
    /// 对应 Java: `LocalLiteflowComponentSupport#getCmpName`。
    #[must_use]
    pub fn get_cmp_name(&self, component: &dyn NodeComponent) -> Option<String> {
        let _ = component;
        None
    }

    /// 返回本地组件名称解析器的 SPI 优先级。
    ///
    /// 返回值为 `2`。对应 Java: `LocalLiteflowComponentSupport#priority`。
    #[must_use]
    pub fn priority(&self) -> i32 {
        2
    }
}

impl LiteflowComponentSupport for LocalLiteflowComponentSupport {
    fn get_cmp_name(&self, component: &dyn NodeComponent) -> Option<String> {
        LocalLiteflowComponentSupport::get_cmp_name(self, component)
    }
}

impl SpiPriority for LocalLiteflowComponentSupport {
    fn priority(&self) -> i32 {
        LocalLiteflowComponentSupport::priority(self)
    }
}
