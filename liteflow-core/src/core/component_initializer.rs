//! 组件初始化器。
//!
//! 对应 Java: `com.yomahub.liteflow.core.ComponentInitializer`。

use std::sync::{Arc, OnceLock};

use crate::enums::NodeTypeEnum;
use crate::exception::{LFResult, LiteflowError};
use crate::flow::executor::NodeExecutor;
use crate::spi::LiteflowComponentSupportHolder;

use super::NodeComponent;
use super::initialized_node_component::InitializedNodeComponent;

/// 将节点身份、类型、名称、默认重试和默认执行器注入不可变 Rust 组件。
///
/// Java 直接调用 `NodeComponent#set*` 修改 Spring Bean；Rust 使用不可变委托，
/// 既保留共享 `Arc` 的线程安全，又让所有注册入口复用同一初始化算法。
#[derive(Clone, Default)]
pub struct ComponentInitializer {
    /// 显式覆盖的全局重试次数；`None` 时在组件执行期读取 LiteflowConfigGetter。
    default_retry_count: Option<usize>,
    default_node_executor: Option<Arc<dyn NodeExecutor>>,
}

impl ComponentInitializer {
    /// 返回进程级默认初始化器。
    ///
    /// 对应 Java: `ComponentInitializer#loadInstance`。
    #[must_use]
    pub fn load_instance() -> &'static Self {
        static INSTANCE: OnceLock<ComponentInitializer> = OnceLock::new();
        INSTANCE.get_or_init(Self::default)
    }

    /// 创建使用指定全局重试次数的初始化器。
    ///
    /// 对应 Java `LiteflowConfig#getRetryCount` 的全局回退分支。
    #[must_use]
    pub fn with_default_retry_count(default_retry_count: usize) -> Self {
        Self {
            default_retry_count: Some(default_retry_count),
            default_node_executor: None,
        }
    }

    /// 设置全局默认节点执行器。
    ///
    /// 对应 Java: `ComponentInitializer#buildNodeExecutorClass`；Rust 直接保存
    /// `Arc<dyn NodeExecutor>`，替代 class name 反射与容器实例化。
    #[must_use]
    pub fn with_default_node_executor(
        mut self,
        default_node_executor: Arc<dyn NodeExecutor>,
    ) -> Self {
        self.default_node_executor = Some(default_node_executor);
        self
    }

    /// 初始化并返回线程安全的组件委托。
    ///
    /// `node_component`、`node_type`、`name`、`node_id` 分别对应 Java
    /// `initComponent(NodeComponent, NodeTypeEnum, String, String)` 的同名参数。
    pub fn init_component(
        &self,
        node_component: Arc<dyn NodeComponent>,
        node_type: NodeTypeEnum,
        name: Option<&str>,
        node_id: &str,
    ) -> LFResult<Arc<dyn NodeComponent>> {
        self.init_component_with_type_source(node_component, node_type, true, name, node_id)
    }

    /// 初始化由 Rust 闭包便利入口注册、节点类型需要从 EL 位置推断的组件。
    pub(crate) fn init_inferred_component(
        &self,
        node_component: Arc<dyn NodeComponent>,
        node_type: NodeTypeEnum,
        name: Option<&str>,
        node_id: &str,
    ) -> LFResult<Arc<dyn NodeComponent>> {
        self.init_component_with_type_source(node_component, node_type, false, name, node_id)
    }

    /// 共用不可变委托初始化，并记录节点类型是显式声明还是位置推断。
    fn init_component_with_type_source(
        &self,
        node_component: Arc<dyn NodeComponent>,
        node_type: NodeTypeEnum,
        node_type_explicit: bool,
        name: Option<&str>,
        node_id: &str,
    ) -> LFResult<Arc<dyn NodeComponent>> {
        let node_id = node_id.trim();
        if node_id.is_empty() {
            return Err(LiteflowError::NodeBuild("[id is blank]".to_string()));
        }
        let configured_name = name.map(str::trim).unwrap_or_default();
        let component_name = if configured_name.is_empty() && !node_type.is_script() {
            // Java 先采用规则文件显式 name；普通组件仍为空时，再由当前容器的
            // LiteflowComponentSupport 解析 @LiteflowComponent 名称。
            LiteflowComponentSupportHolder::load_liteflow_component_support()
                .get_cmp_name(node_component.as_ref())
                .unwrap_or_default()
        } else {
            configured_name.to_string()
        };

        // 名称已经按 Java 顺序完成“规则配置 → 容器注解 SPI”解析；
        // 重试次数同样优先采用组件声明，委托对象只保存全局缺省值。
        Ok(Arc::new(InitializedNodeComponent::new(
            node_component,
            node_id.to_string(),
            node_type,
            node_type_explicit,
            component_name,
            self.default_retry_count,
            self.default_node_executor.clone(),
        )))
    }
}
