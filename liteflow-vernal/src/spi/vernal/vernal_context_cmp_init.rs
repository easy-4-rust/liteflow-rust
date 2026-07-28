//! 对应 Java 类：com.yomahub.liteflow.spi.spring.SpringContextCmpInit

use std::sync::Arc;

use liteflow_core::FlowBus;
use liteflow_core::core::NodeComponent;
use liteflow_core::exception::LFResult;
use liteflow_core::spi::{ContextCmpInit, SpiPriority};

/// Vernal 容器上下文中的组件初始化 SPI。
///
/// Java 从 `SpringNodeIdHolder` 读取扫描得到的节点 ID，再由 `FlowBus` 从 Spring
/// 容器取得同一 Bean 并调用 `addManagedNode`。Vernal 不进行 classpath 反射扫描，
/// 因此模块在注册期把类型擦除后的真实节点实例连同 ID 交给本对象；初始化时仍
/// 统一走 `FlowBus::add_managed_node`，执行节点类型判断、元数据注入和注册校验。
///
/// 对应 Java: `com.yomahub.liteflow.spi.spring.SpringContextCmpInit`。
pub struct VernalContextCmpInit {
    flow_bus: FlowBus,
    managed_nodes: Vec<(String, Arc<dyn NodeComponent>)>,
}

impl VernalContextCmpInit {
    /// 创建 Vernal 托管组件初始化器。
    ///
    /// # 参数
    /// - `flow_bus`：目标 LiteFlow 注册总线；
    /// - `managed_nodes`：Vernal 容器托管的节点 ID 与真实实例。
    ///
    /// # 返回
    /// 尚未执行组件装配的初始化器。对应 Java:
    /// `SpringContextCmpInit#SpringContextCmpInit`。
    #[must_use]
    pub fn new(flow_bus: FlowBus, managed_nodes: Vec<(String, Arc<dyn NodeComponent>)>) -> Self {
        Self {
            flow_bus,
            managed_nodes,
        }
    }

    /// 尝试初始化全部 Vernal 托管节点。
    ///
    /// # 返回
    /// 所有节点完成类型推断和注册时返回 `Ok(())`；任一节点不合法时返回核心
    /// LiteFlow 错误。对应 Java: `SpringContextCmpInit#initCmp`。
    pub fn try_init_cmp(&self) -> LFResult<()> {
        for (node_id, node_component) in &self.managed_nodes {
            // 使用同一个 Arc 实例进入 FlowBus，保持 Vernal IoC 与 LiteFlow 执行
            // 期间观察到的业务状态一致。
            self.flow_bus
                .add_managed_node(node_id.clone(), Arc::clone(node_component))?;
        }
        Ok(())
    }

    /// 初始化全部 Vernal 托管节点。
    ///
    /// Java SPI 使用 `void` 并以运行时异常报告非法组件；Rust trait 为保持方法
    /// 对齐也使用无返回入口，详细错误由模块装配阶段的 `try_init_cmp` 提前处理。
    /// 对应 Java: `SpringContextCmpInit#initCmp`。
    pub fn init_cmp(&self) {
        self.try_init_cmp()
            .expect("Vernal managed component initialization failed");
    }

    /// 返回当前初始化器持有的托管节点数量。
    ///
    /// # 返回
    /// 待初始化节点的数量，用于容器诊断和测试。
    #[must_use]
    pub fn managed_node_count(&self) -> usize {
        self.managed_nodes.len()
    }

    /// 返回容器实现的 SPI 优先级。
    ///
    /// # 返回
    /// 固定返回 `1`，优先于本地空实现。对应 Java:
    /// `SpringContextCmpInit#priority`。
    #[must_use]
    pub fn priority(&self) -> i32 {
        1
    }
}

impl ContextCmpInit for VernalContextCmpInit {
    fn init_cmp(&self) {
        VernalContextCmpInit::init_cmp(self);
    }
}

impl SpiPriority for VernalContextCmpInit {
    fn priority(&self) -> i32 {
        VernalContextCmpInit::priority(self)
    }
}
