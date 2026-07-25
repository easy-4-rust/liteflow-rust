//! 对应 Java 类：com.yomahub.liteflow.spi.local.LocalLiteflowComponentSupport
//!
//! 非 spring 环境 LiteflowComponent 注解的处理器。
//! Java 版返回 null（非 spring 环境不支持 @LiteflowComponent 注解）；
//! Rust 无注解体系，组件名直接取自 NodeComponent 自身（name()），
//! 即「本地环境下组件声明名就是其注册名」。

use crate::core::NodeComponent;
use crate::spi::liteflow_component_support::LiteflowComponentSupport;
use crate::spi::spi_priority::SpiPriority;

/// 对应 LocalLiteflowComponentSupport
#[derive(Default)]
pub struct LocalLiteflowComponentSupport;

impl LocalLiteflowComponentSupport {
    pub fn new() -> Self {
        Self
    }
}

impl LiteflowComponentSupport for LocalLiteflowComponentSupport {
    /// 对应 getCmpName(NodeComponent nodeComponent)
    fn get_cmp_name(&self, component: &dyn NodeComponent) -> Option<String> {
        Some(component.name().to_string())
    }
}

impl SpiPriority for LocalLiteflowComponentSupport {
    /// 对应 priority()
    fn priority(&self) -> i32 {
        2
    }
}
