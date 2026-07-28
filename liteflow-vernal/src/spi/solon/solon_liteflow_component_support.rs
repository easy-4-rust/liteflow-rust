use liteflow_core::core::NodeComponent;
use liteflow_core::spi::{LiteflowComponentSupport, SpiPriority};

/// Solon 环境的 `LiteflowComponent` 名称处理器。
///
/// Java 从节点类的 `@LiteflowComponent(name=...)` 读取名称；Rust 过程宏把同一
/// 元数据暴露为 `NodeComponent::name`，空名称映射 Java `null`。对应 Java:
/// `com.yomahub.liteflow.spi.solon.SolonLiteflowComponentSupport`。
#[derive(Debug, Default)]
pub struct SolonLiteflowComponentSupport;

impl SolonLiteflowComponentSupport {
    /// 创建无状态的 Solon 组件名称处理器。
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// 返回组件注解声明的显示名称。
    ///
    /// # 参数
    /// - `node_component`：Solon 托管的真实节点。
    ///
    /// # 返回
    /// 非空名称返回 `Some`；未声明名称返回 `None`。对应 Java:
    /// `SolonLiteflowComponentSupport#getCmpName`。
    #[must_use]
    pub fn get_cmp_name(&self, node_component: &dyn NodeComponent) -> Option<String> {
        let component_name = node_component.name().trim();
        (!component_name.is_empty()).then(|| component_name.to_string())
    }

    /// 返回 Solon SPI 优先级。
    #[must_use]
    pub fn priority(&self) -> i32 {
        1
    }
}

impl LiteflowComponentSupport for SolonLiteflowComponentSupport {
    fn get_cmp_name(&self, component: &dyn NodeComponent) -> Option<String> {
        SolonLiteflowComponentSupport::get_cmp_name(self, component)
    }
}

impl SpiPriority for SolonLiteflowComponentSupport {
    fn priority(&self) -> i32 {
        SolonLiteflowComponentSupport::priority(self)
    }
}
