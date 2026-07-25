//! 对应 Java 类：com.yomahub.liteflow.spi.LiteflowComponentSupport
//!
//! LiteflowComponent 注解处理器 SPI 接口。
//! Java 返回 String（可能为 null，表示注解体系未提供名称）；
//! Rust 以 `Option<String>` 表达可空语义。

use crate::core::NodeComponent;

use super::spi_priority::SpiPriority;

/// 对应 LiteflowComponentSupport
pub trait LiteflowComponentSupport: SpiPriority + Send + Sync {
    /// 对应 getCmpName(NodeComponent nodeComponent)
    fn get_cmp_name(&self, component: &dyn NodeComponent) -> Option<String>;
}
