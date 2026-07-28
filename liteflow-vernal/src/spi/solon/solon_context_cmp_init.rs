use std::sync::Arc;

use liteflow_core::FlowBus;
use liteflow_core::exception::LFResult;
use liteflow_core::spi::{ContextCmpInit, SpiPriority};

use crate::process::holder::SolonNodeIdHolder;

use super::SolonContextAware;

/// Solon 容器上下文中的组件初始化 SPI。
///
/// Java 从当前 `AppContext` 附件的 `SolonNodeIdHolder` 逐个调用
/// `FlowBus.addManagedNode`。Rust 保存同一上下文适配器、Holder 与 FlowBus，
/// 通过上下文的节点 trait-object 表取得真实单例。对应 Java:
/// `com.yomahub.liteflow.spi.solon.SolonContextCmpInit`。
pub struct SolonContextCmpInit {
    flow_bus: FlowBus,
    context: Arc<SolonContextAware>,
    node_id_holder: Arc<SolonNodeIdHolder>,
}

impl SolonContextCmpInit {
    /// 创建 Solon 容器组件初始化器。
    ///
    /// # 参数
    /// - `flow_bus`：目标流程总线；
    /// - `context`：当前 Solon 上下文；
    /// - `node_id_holder`：当前上下文独享的节点 ID 集合。
    #[must_use]
    pub fn new(
        flow_bus: FlowBus,
        context: Arc<SolonContextAware>,
        node_id_holder: Arc<SolonNodeIdHolder>,
    ) -> Self {
        Self {
            flow_bus,
            context,
            node_id_holder,
        }
    }

    /// 尝试初始化 Holder 中的全部容器节点。
    ///
    /// # 返回
    /// 全部真实节点成功进入 FlowBus 时返回 `Ok(())`；Holder 中存在无对应 Bean
    /// 的 ID 时返回配置错误。对应 Java: `SolonContextCmpInit#initCmp`。
    pub fn try_init_cmp(&self) -> LFResult<()> {
        let mut node_ids = self
            .node_id_holder
            .get_node_id_set()
            .into_iter()
            .collect::<Vec<_>>();
        node_ids.sort();
        for node_id in node_ids {
            let node_component = self.context.get_node_component(&node_id).ok_or_else(|| {
                liteflow_core::LiteflowError::CmpDefinition(format!(
                    "Solon managed component[{node_id}] is not registered"
                ))
            })?;
            // FlowBus 与 Solon 上下文持有同一个 Arc，节点内部状态不会被复制。
            self.flow_bus.add_managed_node(node_id, node_component)?;
        }
        Ok(())
    }

    /// 初始化 Holder 中的全部容器节点。
    ///
    /// Java 接口返回 `void` 并用运行时异常报告错误；Rust trait 保留无返回签名，
    /// 插件装配阶段则调用 `try_init_cmp` 获得结构化错误。
    pub fn init_cmp(&self) {
        self.try_init_cmp()
            .expect("Solon managed component initialization failed");
    }

    /// 返回待初始化节点数量。
    ///
    /// # 返回
    /// 当前上下文 Holder 的去重节点数。
    #[must_use]
    pub fn managed_node_count(&self) -> usize {
        self.node_id_holder.get_node_id_set().len()
    }

    /// 返回 Solon SPI 优先级。
    ///
    /// # 返回
    /// 固定为 `1`。对应 Java: `priority`。
    #[must_use]
    pub fn priority(&self) -> i32 {
        1
    }
}

impl ContextCmpInit for SolonContextCmpInit {
    fn init_cmp(&self) {
        SolonContextCmpInit::init_cmp(self);
    }
}

impl SpiPriority for SolonContextCmpInit {
    fn priority(&self) -> i32 {
        SolonContextCmpInit::priority(self)
    }
}
