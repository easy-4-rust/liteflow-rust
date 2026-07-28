use std::any::{Any, type_name};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use liteflow_core::core::NodeComponent;
use liteflow_core::core::proxy::DeclWarpBean;
use liteflow_core::spi::{Bean, ContextAware, SpiPriority};

/// Solon 应用上下文的 LiteFlow Bean 访问与动态注册适配器。
///
/// Java 通过 `Solon.context()` 和 `BeanWrap` 完成按名称/类型查询及动态注册；
/// Rust 使用线程安全命名表保存同一共享对象，并额外保留节点 trait-object 表，
/// 解决类型擦除后无法从 `Any` 恢复 `dyn NodeComponent` 的语言差异。对应 Java:
/// `com.yomahub.liteflow.spi.solon.SolonContextAware`。
#[derive(Default)]
pub struct SolonContextAware {
    beans: RwLock<HashMap<String, Bean>>,
    bean_types: RwLock<HashMap<String, &'static str>>,
    node_components: RwLock<HashMap<String, Arc<dyn NodeComponent>>>,
}

impl SolonContextAware {
    /// 创建空的 Solon 上下文适配器。
    ///
    /// # 返回
    /// 尚未登记 Bean 的上下文对象。对应 Java:
    /// `SolonContextAware#SolonContextAware`。
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// 按名称获取 Bean。
    ///
    /// # 参数
    /// - `bean_name`：Solon `BeanWrap#name` 对应的注册名。
    ///
    /// # 返回
    /// 已注册共享对象；查询失败返回 `None`，对应 Java 捕获异常后返回 `null`。
    #[must_use]
    pub fn get_bean(&self, bean_name: &str) -> Option<Bean> {
        self.beans
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(bean_name)
            .cloned()
    }

    /// 注册一个已有 Bean。
    ///
    /// Java 对已存在的 `BeanWrap` 保持首个对象；Rust 使用 `entry` 保留同样语义。
    ///
    /// # 参数
    /// - `bean_name`：注册名；
    /// - `bean`：线程安全共享对象。
    ///
    /// # 返回
    /// 容器中最终保存的对象。对应 Java:
    /// `SolonContextAware#registerBean(String,Object)`。
    pub fn register_bean(&self, bean_name: &str, bean: Bean) -> Bean {
        let mut beans = self
            .beans
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        Arc::clone(
            beans
                .entry(bean_name.to_string())
                .or_insert_with(|| Arc::clone(&bean)),
        )
    }

    /// 注册保留具体 Rust 类型名的 Bean。
    ///
    /// # 参数
    /// - `bean_name`：Solon 注册名；
    /// - `bean`：具体类型共享对象。
    ///
    /// # 返回
    /// 擦除为 LiteFlow `Bean` 后的同一实例。
    pub fn register_typed_bean<T>(&self, bean_name: &str, bean: Arc<T>) -> Bean
    where
        T: Any + Send + Sync,
    {
        if let Some(existing) = self.get_bean(bean_name) {
            return existing;
        }
        let erased: Bean = bean;
        self.beans
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(bean_name.to_string(), Arc::clone(&erased));
        self.bean_types
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(bean_name.to_string(), type_name::<T>());
        erased
    }

    /// 注册 Solon 托管的节点组件。
    ///
    /// # 参数
    /// - `bean_name`：Bean 名，同时作为默认节点 ID；
    /// - `node_component`：容器与 FlowBus 共享的真实节点实例。
    ///
    /// # 返回
    /// 已存在或本次登记的同一节点对象。对应 Java:
    /// `SolonContextAware#registerBean` 与 `XPluginImpl#subWrapsOfType`。
    pub fn register_node_component(
        &self,
        bean_name: &str,
        node_component: Arc<dyn NodeComponent>,
    ) -> Arc<dyn NodeComponent> {
        let mut node_components = self
            .node_components
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        Arc::clone(
            node_components
                .entry(bean_name.to_string())
                .or_insert(node_component),
        )
    }

    /// 返回指定名称的节点组件。
    ///
    /// # 参数
    /// - `bean_name`：节点 Bean 名。
    ///
    /// # 返回
    /// 可直接交给 `FlowBus#addManagedNode` 的共享 trait object。
    #[must_use]
    pub fn get_node_component(&self, bean_name: &str) -> Option<Arc<dyn NodeComponent>> {
        self.node_components
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(bean_name)
            .cloned()
    }

    /// 获取或构造指定名称的 Bean。
    ///
    /// # 参数
    /// - `bean_name`：注册名；
    /// - `factory`：Bean 不存在时的构造器。
    ///
    /// # 返回
    /// 竞争中最终进入上下文的单例。对应 Java:
    /// `SolonContextAware#registerOrGet`。
    pub fn register_or_get(&self, bean_name: &str, factory: &dyn Fn() -> Bean) -> Bean {
        if let Some(bean) = self.get_bean(bean_name) {
            return bean;
        }
        let candidate = factory();
        self.register_bean(bean_name, candidate)
    }

    /// 返回指定类型的全部 Bean。
    ///
    /// # 参数
    /// - `type_name`：完整 Rust 类型名；`None` 返回全部命名 Bean。
    ///
    /// # 返回
    /// 名称到共享对象的快照。对应 Java:
    /// `SolonContextAware#getBeansOfType`。
    #[must_use]
    pub fn get_beans_of_type(&self, type_name: Option<&str>) -> Option<HashMap<String, Bean>> {
        let beans = self
            .beans
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let bean_types = self
            .bean_types
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        Some(
            beans
                .iter()
                .filter(|(bean_name, _)| {
                    type_name.is_none_or(|expected| {
                        bean_types
                            .get(*bean_name)
                            .is_some_and(|actual| *actual == expected)
                    })
                })
                .map(|(bean_name, bean)| (bean_name.clone(), Arc::clone(bean)))
                .collect(),
        )
    }

    /// 判断指定名称的 Bean 是否存在。
    ///
    /// # 参数
    /// - `bean_name`：待检查名称。
    #[must_use]
    pub fn has_bean(&self, bean_name: &str) -> bool {
        self.beans
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .contains_key(bean_name)
    }

    /// 判断指定 Rust 类型是否至少有一个 Bean。
    ///
    /// # 参数
    /// - `type_name`：`std::any::type_name::<T>()` 形式的类型标识。
    #[must_use]
    pub fn has_bean_type(&self, type_name: &str) -> bool {
        self.bean_types
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .values()
            .any(|registered| *registered == type_name)
    }

    /// 注册声明式组件包装对象。
    ///
    /// Rust 的代理实际进入 FlowBus 由 `XPluginImpl` 完成；上下文保存同一
    /// `DeclWarpBean` 元数据，供按名称查询与诊断。对应 Java:
    /// `SolonContextAware#registerDeclWrapBean`。
    pub fn register_decl_wrap_bean(
        &self,
        bean_name: &str,
        decl_warp_bean: DeclWarpBean,
    ) -> Option<Bean> {
        Some(self.register_typed_bean(bean_name, Arc::new(decl_warp_bean)))
    }

    /// 返回 Solon SPI 优先级。
    ///
    /// # 返回
    /// 固定为 `1`。对应 Java: `SolonContextAware#priority`。
    #[must_use]
    pub fn priority(&self) -> i32 {
        1
    }
}

impl ContextAware for SolonContextAware {
    fn get_bean(&self, bean_name: &str) -> Option<Bean> {
        SolonContextAware::get_bean(self, bean_name)
    }

    fn register_bean(&self, bean_name: &str, bean: Bean) -> Bean {
        SolonContextAware::register_bean(self, bean_name, bean)
    }

    fn has_bean(&self, bean_name: &str) -> bool {
        SolonContextAware::has_bean(self, bean_name)
    }

    fn register_or_get(&self, bean_name: &str, factory: &dyn Fn() -> Bean) -> Bean {
        SolonContextAware::register_or_get(self, bean_name, factory)
    }

    fn get_beans_of_type(&self, type_name: Option<&str>) -> Option<HashMap<String, Bean>> {
        SolonContextAware::get_beans_of_type(self, type_name)
    }

    fn has_bean_type(&self, type_name: &str) -> bool {
        SolonContextAware::has_bean_type(self, type_name)
    }

    fn register_decl_wrap_bean(
        &self,
        bean_name: &str,
        decl_warp_bean: DeclWarpBean,
    ) -> Option<Bean> {
        SolonContextAware::register_decl_wrap_bean(self, bean_name, decl_warp_bean)
    }
}

impl SpiPriority for SolonContextAware {
    fn priority(&self) -> i32 {
        SolonContextAware::priority(self)
    }
}
