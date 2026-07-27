//! 对应 Java 类：com.yomahub.liteflow.spi.local.LocalContextAware
//!
//! 非 Spring 环境没有 Bean 容器，因此必须保留 Java 的无状态空实现语义：
//! 查询恒为空、存在性判断恒为 false、注册现有对象只原样返回且不落库。

use std::collections::HashMap;

use crate::core::proxy::DeclWarpBean;
use crate::spi::spi_priority::SpiPriority;
use crate::spi::{Bean, ContextAware};

/// 非容器环境的 `ContextAware` 空实现。
///
/// 该对象不保存任何 Bean；需要真实依赖注入时由 `liteflow-vernal` 提供 Vernal
/// 容器实现。对应 Java: `com.yomahub.liteflow.spi.local.LocalContextAware`。
#[derive(Default)]
pub struct LocalContextAware;

impl LocalContextAware {
    /// 创建无状态的本地环境实现。
    ///
    /// 返回值不持有容器状态。对应 Java: `LocalContextAware#LocalContextAware`。
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// 按名称查询 Bean。
    ///
    /// 参数 `bean_name` 是待查询名称；非容器环境始终返回 `None`，对应 Java
    /// `null`。对应 Java: `LocalContextAware#getBean(String)`。
    #[must_use]
    pub fn get_bean(&self, _bean_name: &str) -> Option<Bean> {
        None
    }

    /// 接受一个现有 Bean，但不把它保存到容器。
    ///
    /// 参数 `bean_name` 仅保留 Java API 语义，返回值是传入的 `bean` 本身。
    /// 对应 Java: `LocalContextAware#registerBean(String, Object)`。
    pub fn register_bean(&self, _bean_name: &str, bean: Bean) -> Bean {
        bean
    }

    /// 通过工厂构造一个 Bean，但不把它保存到容器。
    ///
    /// Rust 的 `factory` 对应 Java `Class<T>` 反射构造；每次调用都会构造新对象。
    /// 参数 `bean_name` 不参与存储。对应 Java:
    /// `LocalContextAware#registerOrGet(String, Class)`。
    pub fn register_or_get(&self, _bean_name: &str, factory: &dyn Fn() -> Bean) -> Bean {
        factory()
    }

    /// 获取指定类型及子类型对应的全部 Bean。
    ///
    /// 参数 `type_name` 为 Rust 类型标识，`None` 表示全部类型；无容器环境始终
    /// 返回 `None`，对应 Java `null`。对应 Java:
    /// `LocalContextAware#getBeansOfType(Class)`。
    #[must_use]
    pub fn get_beans_of_type(&self, _type_name: Option<&str>) -> Option<HashMap<String, Bean>> {
        None
    }

    /// 判断是否存在指定名称的 Bean。
    ///
    /// 参数 `bean_name` 为注册名；无容器环境始终返回 false。对应 Java:
    /// `LocalContextAware#hasBean(String)`。
    #[must_use]
    pub fn has_bean(&self, _bean_name: &str) -> bool {
        false
    }

    /// 判断是否存在指定类型的 Bean。
    ///
    /// 参数 `type_name` 对应 Java `clazz`；无容器环境始终返回 false。
    /// 对应 Java: `LocalContextAware#hasBean(Class)`。
    #[must_use]
    pub fn has_bean_type(&self, _type_name: &str) -> bool {
        false
    }

    /// 接收声明式组件包装对象。
    ///
    /// 非容器环境不注册 Bean，因而始终返回 `None`。参数与 Java API 一一对应。
    /// 对应 Java: `LocalContextAware#registerDeclWrapBean`。
    pub fn register_decl_wrap_bean(
        &self,
        _bean_name: &str,
        _decl_warp_bean: DeclWarpBean,
    ) -> Option<Bean> {
        None
    }

    /// 返回 SPI 优先级 2。
    ///
    /// 数字越小优先级越高，因此真实 Vernal 容器实现应使用更小值。
    /// 对应 Java: `LocalContextAware#priority`。
    #[must_use]
    pub fn priority(&self) -> i32 {
        2
    }
}

impl ContextAware for LocalContextAware {
    fn get_bean(&self, bean_name: &str) -> Option<Bean> {
        LocalContextAware::get_bean(self, bean_name)
    }

    fn register_bean(&self, bean_name: &str, bean: Bean) -> Bean {
        LocalContextAware::register_bean(self, bean_name, bean)
    }

    fn has_bean(&self, bean_name: &str) -> bool {
        LocalContextAware::has_bean(self, bean_name)
    }

    fn register_or_get(&self, bean_name: &str, factory: &dyn Fn() -> Bean) -> Bean {
        LocalContextAware::register_or_get(self, bean_name, factory)
    }

    fn get_beans_of_type(&self, type_name: Option<&str>) -> Option<HashMap<String, Bean>> {
        LocalContextAware::get_beans_of_type(self, type_name)
    }

    fn has_bean_type(&self, type_name: &str) -> bool {
        LocalContextAware::has_bean_type(self, type_name)
    }

    fn register_decl_wrap_bean(
        &self,
        bean_name: &str,
        decl_warp_bean: DeclWarpBean,
    ) -> Option<Bean> {
        LocalContextAware::register_decl_wrap_bean(self, bean_name, decl_warp_bean)
    }
}

impl SpiPriority for LocalContextAware {
    fn priority(&self) -> i32 {
        LocalContextAware::priority(self)
    }
}
