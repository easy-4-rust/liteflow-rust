//! 对应 Java 类：com.yomahub.liteflow.spi.spring.SpringAware

use std::any::{Any, type_name};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use liteflow_core::core::proxy::DeclWarpBean;
use liteflow_core::spi::{Bean, ContextAware, SpiPriority};

/// Vernal 容器的 LiteFlow 命名 Bean 适配器。
///
/// Vernal 的主注册表在应用构建后保持不可变，本对象作为真实 Vernal 单例组件，
/// 以线程安全命名表承接 Java `DefaultListableBeanFactory#registerSingleton` 的动态
/// 注册语义。模块创建的 `FlowBus`、`LiteflowRuntime` 与配置对象也进入同一张表，
/// 因而 LiteFlow SPI 查询和 Vernal 类型解析指向同一批真实实例。
///
/// 对应 Java: `com.yomahub.liteflow.spi.spring.SpringAware`。
#[derive(Default)]
pub struct VernalAware {
    beans: RwLock<HashMap<String, Bean>>,
    bean_types: RwLock<HashMap<String, &'static str>>,
}

impl VernalAware {
    /// 创建空的 Vernal 命名 Bean 适配器。
    ///
    /// # 返回
    /// 尚未登记 Bean 的容器适配对象。对应 Java:
    /// `SpringAware#SpringAware`。
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// 按注册名称获取 Bean。
    ///
    /// # 参数
    /// - `bean_name`：Vernal/LiteFlow 共享的稳定注册名。
    ///
    /// # 返回
    /// 已登记 Bean 的共享句柄，不存在时返回 `None`。对应 Java:
    /// `SpringAware#getBean(String)`。
    #[must_use]
    pub fn get_bean(&self, bean_name: &str) -> Option<Bean> {
        self.beans
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(bean_name)
            .cloned()
    }

    /// 注册一个已擦除类型的 Bean。
    ///
    /// # 参数
    /// - `bean_name`：注册名；
    /// - `bean`：线程安全共享对象。
    ///
    /// # 返回
    /// 容器内保存的同一共享对象。对应 Java:
    /// `SpringAware#registerBean(String, Object)`。
    pub fn register_bean(&self, bean_name: &str, bean: Bean) -> Bean {
        self.bean_types
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .remove(bean_name);
        self.beans
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(bean_name.to_string(), Arc::clone(&bean));
        bean
    }

    /// 注册一个保留 Rust 类型名的 Bean。
    ///
    /// 该 Rust 固有入口补足 Java 可从 `Object#getClass` 获得运行时类型、而
    /// `Arc<dyn Any>` 无法反推出具体类型名的语言差异。
    ///
    /// # 参数
    /// - `bean_name`：注册名；
    /// - `bean`：具体类型的共享对象。
    ///
    /// # 返回
    /// 擦除为 LiteFlow `Bean` 后的同一共享对象。
    pub fn register_typed_bean<T>(&self, bean_name: &str, bean: Arc<T>) -> Bean
    where
        T: Any + Send + Sync,
    {
        let bean: Bean = bean;
        self.beans
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(bean_name.to_string(), Arc::clone(&bean));
        self.bean_types
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(bean_name.to_string(), type_name::<T>());
        bean
    }

    /// 获取或构造指定名称的 Bean。
    ///
    /// # 参数
    /// - `bean_name`：稳定注册名；
    /// - `factory`：Bean 不存在时调用的 Rust 构造器。
    ///
    /// # 返回
    /// 已有对象或本次竞争中最终写入的对象。对应 Java:
    /// `SpringAware#registerOrGet(String, Class)`。
    pub fn register_or_get(&self, bean_name: &str, factory: &dyn Fn() -> Bean) -> Bean {
        if let Some(bean) = self.get_bean(bean_name) {
            return bean;
        }

        // 构造器在锁外执行，避免用户工厂重入 ContextAware 时造成死锁。
        let candidate = factory();
        let mut beans = self
            .beans
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        Arc::clone(
            beans
                .entry(bean_name.to_string())
                .or_insert_with(|| Arc::clone(&candidate)),
        )
    }

    /// 返回指定类型的全部 Bean。
    ///
    /// # 参数
    /// - `type_name`：完整 Rust 类型名；`None` 表示返回全部命名 Bean。
    ///
    /// # 返回
    /// 名称到共享对象的快照。对应 Java:
    /// `SpringAware#getBeansOfType(Class)`。
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

    /// 判断是否存在指定名称的 Bean。
    ///
    /// # 参数
    /// - `bean_name`：待检查注册名。
    ///
    /// # 返回
    /// 存在返回 `true`。对应 Java: `SpringAware#hasBean(String)`。
    #[must_use]
    pub fn has_bean(&self, bean_name: &str) -> bool {
        self.beans
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .contains_key(bean_name)
    }

    /// 判断是否存在指定 Rust 类型名的 Bean。
    ///
    /// # 参数
    /// - `type_name`：`std::any::type_name::<T>()` 形式的完整类型名。
    ///
    /// # 返回
    /// 至少存在一个同类型 Bean 时返回 `true`。对应 Java:
    /// `SpringAware#hasBean(Class)`。
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
    /// # 参数
    /// - `bean_name`：声明式组件注册名；
    /// - `decl_warp_bean`：节点、方法与原始对象元数据。
    ///
    /// # 返回
    /// 已注册包装对象的共享句柄。对应 Java:
    /// `SpringAware#registerDeclWrapBean`。
    pub fn register_decl_wrap_bean(
        &self,
        bean_name: &str,
        decl_warp_bean: DeclWarpBean,
    ) -> Option<Bean> {
        Some(self.register_typed_bean(bean_name, Arc::new(decl_warp_bean)))
    }

    /// 返回 Vernal ContextAware 的 SPI 优先级。
    ///
    /// # 返回
    /// 固定返回 `1`，优先于无容器本地实现。对应 Java:
    /// `SpringAware#priority`。
    #[must_use]
    pub fn priority(&self) -> i32 {
        1
    }
}

impl ContextAware for VernalAware {
    fn get_bean(&self, bean_name: &str) -> Option<Bean> {
        VernalAware::get_bean(self, bean_name)
    }

    fn register_bean(&self, bean_name: &str, bean: Bean) -> Bean {
        VernalAware::register_bean(self, bean_name, bean)
    }

    fn has_bean(&self, bean_name: &str) -> bool {
        VernalAware::has_bean(self, bean_name)
    }

    fn register_or_get(&self, bean_name: &str, factory: &dyn Fn() -> Bean) -> Bean {
        VernalAware::register_or_get(self, bean_name, factory)
    }

    fn get_beans_of_type(&self, type_name: Option<&str>) -> Option<HashMap<String, Bean>> {
        VernalAware::get_beans_of_type(self, type_name)
    }

    fn has_bean_type(&self, type_name: &str) -> bool {
        VernalAware::has_bean_type(self, type_name)
    }

    fn register_decl_wrap_bean(
        &self,
        bean_name: &str,
        decl_warp_bean: DeclWarpBean,
    ) -> Option<Bean> {
        VernalAware::register_decl_wrap_bean(self, bean_name, decl_warp_bean)
    }
}

impl SpiPriority for VernalAware {
    fn priority(&self) -> i32 {
        VernalAware::priority(self)
    }
}
