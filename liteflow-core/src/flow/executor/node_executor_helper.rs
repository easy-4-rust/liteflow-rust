//! 对应 Java 类：com.yomahub.liteflow.flow.executor.NodeExecutorHelper
//!
//! 节点执行器帮助器：单例 + 执行器缓存。
//! 对应关系：
//! - Holder 静态内部类单例 → OnceLock；
//! - ConcurrentHashMap 缓存 → DashMap<TypeId, Arc<dyn NodeExecutor>>
//!   （按执行器类型缓存实例，语义同 Java 按 Class 缓存）；
//! - Java 通过 ContextAwareHolder.registerBean 实例化执行器（DI 容器注册），
//!   Rust 端无 DI 容器：组件指定的自定义执行器由组件直接以 Arc 实例提供
//!   （见 NodeComponent::node_executor），此处仅对默认执行器做单例缓存。

use crate::flow::executor::default_node_executor::DefaultNodeExecutor;
use crate::flow::executor::node_executor::NodeExecutor;
use dashmap::DashMap;
use std::any::TypeId;
use std::sync::{Arc, OnceLock};

/// 节点执行器帮助器（对应 NodeExecutorHelper）
pub struct NodeExecutorHelper {
    /// 执行器实例缓存（对应 nodeExecutorMap）
    node_executor_map: DashMap<TypeId, Arc<dyn NodeExecutor>>,
}

impl NodeExecutorHelper {
    fn new() -> Self {
        Self {
            node_executor_map: DashMap::new(),
        }
    }

    /// 对应 loadInstance()：获取帮助器的单例实例
    pub fn load_instance() -> &'static Self {
        static INSTANCE: OnceLock<NodeExecutorHelper> = OnceLock::new();
        INSTANCE.get_or_init(NodeExecutorHelper::new)
    }

    /// 对应 buildNodeExecutor(Class)：
    /// 组件未指定执行器（None）时返回缓存的 DefaultNodeExecutor；
    /// 组件指定了自定义执行器时直接使用该实例（对应 registerBean 的产物）。
    pub fn build_node_executor(
        &self,
        node_executor: Option<Arc<dyn NodeExecutor>>,
    ) -> Arc<dyn NodeExecutor> {
        match node_executor {
            Some(executor) => executor,
            None => self
                .node_executor_map
                .entry(TypeId::of::<DefaultNodeExecutor>())
                .or_insert_with(|| Arc::new(DefaultNodeExecutor))
                .clone(),
        }
    }
}
