//! 对应 Java 类：com.yomahub.liteflow.spi.spring.SpringLiteflowComponentSupport

use liteflow_core::core::NodeComponent;
use liteflow_core::spi::{LiteflowComponentSupport, SpiPriority};

/// Vernal 环境中的 LiteFlow 组件名称解析器。
///
/// Java 从组件类的 `@LiteflowComponent(name=...)` 读取名称；Rust 过程宏和显式
/// Vernal 组件把相同元数据暴露为 `NodeComponent::name`。空名称返回 `None`，
/// 对应 Java 未标注 `@LiteflowComponent` 时返回 `null`。
///
/// 对应 Java:
/// `com.yomahub.liteflow.spi.spring.SpringLiteflowComponentSupport`。
#[derive(Debug, Default)]
pub struct VernalLiteflowComponentSupport;

impl VernalLiteflowComponentSupport {
    /// 创建 Vernal 组件名称解析器。
    ///
    /// # 返回
    /// 无状态、可跨线程共享的解析器。对应 Java:
    /// `SpringLiteflowComponentSupport#SpringLiteflowComponentSupport`。
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// 读取组件声明的显示名称。
    ///
    /// # 参数
    /// - `node_component`：Vernal 容器持有的真实节点组件。
    ///
    /// # 返回
    /// 非空名称返回 `Some`；未声明名称返回 `None`。对应 Java:
    /// `SpringLiteflowComponentSupport#getCmpName(Object)`。
    #[must_use]
    pub fn get_cmp_name(&self, node_component: &dyn NodeComponent) -> Option<String> {
        let component_name = node_component.name().trim();
        (!component_name.is_empty()).then(|| component_name.to_string())
    }

    /// 返回 Vernal 容器实现的 SPI 优先级。
    ///
    /// # 返回
    /// 固定返回 `1`，优先于本地实现。对应 Java:
    /// `SpringLiteflowComponentSupport#priority`。
    #[must_use]
    pub fn priority(&self) -> i32 {
        1
    }
}

impl LiteflowComponentSupport for VernalLiteflowComponentSupport {
    fn get_cmp_name(&self, component: &dyn NodeComponent) -> Option<String> {
        VernalLiteflowComponentSupport::get_cmp_name(self, component)
    }
}

impl SpiPriority for VernalLiteflowComponentSupport {
    fn priority(&self) -> i32 {
        VernalLiteflowComponentSupport::priority(self)
    }
}
