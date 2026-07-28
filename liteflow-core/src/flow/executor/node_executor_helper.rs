//! 对应 Java 类：com.yomahub.liteflow.flow.executor.NodeExecutorHelper
//!
//! 节点执行器帮助器：单例 + 执行器缓存。
//! 对应关系：
//! - Holder 静态内部类单例 → OnceLock；
//! - ConcurrentHashMap 缓存 → DashMap<TypeId, Arc<dyn NodeExecutor>>
//!   （按执行器类型缓存实例，语义同 Java 按 Class 缓存）；
//! - Java 通过 ContextAwareHolder.registerBean 实例化执行器（DI 容器注册），
//!   Rust 通过按 Java 类名注册的 `Arc<dyn NodeExecutor>` 解析配置，并允许组件
//!   直接提供实例（见 NodeComponent::node_executor）。

use crate::exception::{LFResult, LiteflowError};
use crate::flow::executor::default_node_executor::DefaultNodeExecutor;
use crate::flow::executor::node_executor::NodeExecutor;
use crate::property::LiteflowConfigGetter;
use dashmap::DashMap;
use std::any::TypeId;
use std::sync::{Arc, OnceLock};

const DEFAULT_NODE_EXECUTOR_CLASS: &str = "com.yomahub.liteflow.flow.executor.DefaultNodeExecutor";

/// 缓存并解析节点执行器。
///
/// Java 通过 `Class` 与容器创建执行器；Rust 通过显式类名注册表承接相同扩展点，
/// 默认 Java 类名直接映射为 `DefaultNodeExecutor`。
///
/// 对应 Java: `com.yomahub.liteflow.flow.executor.NodeExecutorHelper`。
pub struct NodeExecutorHelper {
    /// 执行器实例缓存（对应 nodeExecutorMap）
    node_executor_map: DashMap<TypeId, Arc<dyn NodeExecutor>>,
    /// Java 类名到 Rust 执行器实例的显式映射。
    named_node_executor_map: DashMap<String, Arc<dyn NodeExecutor>>,
}

impl NodeExecutorHelper {
    fn new() -> Self {
        Self {
            node_executor_map: DashMap::new(),
            named_node_executor_map: DashMap::new(),
        }
    }

    /// 获取帮助器的进程级单例。
    ///
    /// # 返回
    /// 全局唯一的节点执行器帮助器。
    ///
    /// 对应 Java: `NodeExecutorHelper#loadInstance`。
    pub fn load_instance() -> &'static Self {
        static INSTANCE: OnceLock<NodeExecutorHelper> = OnceLock::new();
        INSTANCE.get_or_init(NodeExecutorHelper::new)
    }

    /// 构建组件直接指定的执行器，未指定时返回缓存的默认执行器。
    ///
    /// 参数 `node_executor` 对应 Java 的 `nodeExecutorClass` 经容器实例化后的对象。
    ///
    /// # 返回
    /// 组件实例或进程内复用的 `DefaultNodeExecutor`。
    ///
    /// 对应 Java: `NodeExecutorHelper#buildNodeExecutor`。
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

    /// 使用 Java 类名注册 Rust 节点执行器。
    ///
    /// 参数 `class_name` 对应 `LiteflowConfig.nodeExecutorClass`；参数
    /// `node_executor` 是替代 Java 容器 `registerBean` 产物的共享 Rust 实例。
    /// 同名注册会原子替换旧实例。
    ///
    /// 对应 Java: `NodeExecutorHelper#buildNodeExecutor` 中的容器 Bean 创建与缓存。
    pub fn register_named_node_executor(
        &self,
        class_name: impl Into<String>,
        node_executor: Arc<dyn NodeExecutor>,
    ) {
        let class_name = class_name.into();
        self.named_node_executor_map
            .insert(class_name.trim().to_string(), node_executor);
    }

    /// 移除指定 Java 类名对应的 Rust 节点执行器。
    ///
    /// 参数 `class_name` 对应注册时使用的完整类名。
    ///
    /// # 返回
    /// 存在并移除时返回 `true`，否则返回 `false`。
    ///
    /// 该方法用于扩展卸载与测试隔离，是 Java 容器销毁语义的 Rust 对应实现。
    pub fn remove_named_node_executor(&self, class_name: &str) -> bool {
        self.named_node_executor_map
            .remove(class_name.trim())
            .is_some()
    }

    /// 按组件覆盖或全局配置解析真实节点执行器。
    ///
    /// 组件直接提供的 `node_executor` 优先；否则读取
    /// `LiteflowConfig.nodeExecutorClass`。默认 Java 类名映射到缓存的默认执行器，
    /// 自定义类名必须先通过 `register_named_node_executor` 注册，未知类名返回
    /// `NodeClassNotFound`，禁止静默降级。
    ///
    /// # 返回
    /// 成功时返回解析后的共享执行器，类名无法解析时返回错误。
    ///
    /// 对应 Java: `ComponentInitializer#buildNodeExecutorClass` 与
    /// `NodeExecutorHelper#buildNodeExecutor`。
    pub fn try_build_node_executor(
        &self,
        node_executor: Option<Arc<dyn NodeExecutor>>,
    ) -> LFResult<Arc<dyn NodeExecutor>> {
        if let Some(node_executor) = node_executor {
            return Ok(node_executor);
        }

        let class_name = LiteflowConfigGetter::get()
            .get_node_executor_class()
            .trim()
            .to_string();
        if class_name.is_empty() || class_name == DEFAULT_NODE_EXECUTOR_CLASS {
            return Ok(self.build_node_executor(None));
        }

        // Java 在组件初始化时通过 Class.forName 校验类名，并由容器创建 Bean；
        // Rust 没有 JVM 反射，因此显式注册表既承担类型解析，也承担实例缓存。
        self.named_node_executor_map
            .get(&class_name)
            .map(|entry| entry.value().clone())
            .ok_or_else(|| {
                LiteflowError::NodeClassNotFound(format!(
                    "node executor class[{class_name}] is not registered"
                ))
            })
    }
}
