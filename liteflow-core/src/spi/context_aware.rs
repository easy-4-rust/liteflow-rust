//! 对应 Java 类：com.yomahub.liteflow.spi.ContextAware
//!
//! 环境容器 SPI 接口。Java 版基于泛型 `<T> T getBean(String name)` 从
//! Spring/本地容器中取 Bean；Rust 无运行时泛型擦除容器，统一 Rust 化为
//! 基于 `Arc<dyn Any + Send + Sync>` 的 bean 容器，取用时由调用方 downcast。

use std::collections::HashMap;

use crate::core::proxy::DeclWarpBean;

use super::Bean;
use super::spi_priority::SpiPriority;

/// 环境容器 SPI 接口。
///
/// Java 通过重载方法同时支持名称和类型查找；Rust 使用 `Bean` 与显式类型名表达
/// 相同边界。非容器环境可以返回 `None`，Vernal 实现则负责真实对象注册与查询。
///
/// 对应 Java: `com.yomahub.liteflow.spi.ContextAware`。
pub trait ContextAware: SpiPriority + Send + Sync {
    /// 按注册名称获取 Bean。
    ///
    /// 参数 `bean_name` 是容器注册名；返回 `None` 表示不存在或当前实现没有容器。
    /// 对应 Java: `ContextAware#getBean(String)`。
    fn get_bean(&self, bean_name: &str) -> Option<Bean>;

    /// 注册现有 Bean 并返回容器中的对象。
    ///
    /// 参数 `bean_name`、`bean` 分别对应 Java 同名参数。对应 Java:
    /// `ContextAware#registerBean(String, Object)`。
    fn register_bean(&self, bean_name: &str, bean: Bean) -> Bean;

    /// 判断是否存在指定名称的 Bean。
    ///
    /// 参数 `bean_name` 为容器注册名。对应 Java:
    /// `ContextAware#hasBean(String)`。
    fn has_bean(&self, bean_name: &str) -> bool;

    /// 获取或构造指定名称的 Bean。
    ///
    /// Java 通过 `Class<T>` 反射实例化；Rust 由 `factory` 承担等价构造职责。
    /// 参数 `bean_name` 对应 Java 同名参数。对应 Java:
    /// `ContextAware#registerOrGet(String, Class)`。
    fn register_or_get(&self, bean_name: &str, factory: &dyn Fn() -> Bean) -> Bean;

    /// 获取指定类型及其子类型对应的全部 Bean。
    ///
    /// 参数 `type_name` 为 Rust 类型标识，`None` 对应 Java 的 `null`，表示获取全部
    /// Bean；返回 Map 的 key 是注册名。默认 `None` 对应无容器实现返回 `null`。
    /// 对应 Java: `ContextAware#getBeansOfType(Class)`。
    fn get_beans_of_type(&self, _type_name: Option<&str>) -> Option<HashMap<String, Bean>> {
        None
    }

    /// 判断是否存在指定类型的 Bean。
    ///
    /// 参数 `type_name` 对应 Java `clazz`；默认实现用于无容器环境。
    /// 对应 Java: `ContextAware#hasBean(Class)`。
    fn has_bean_type(&self, _type_name: &str) -> bool {
        false
    }

    /// 注册声明式组件包装对象。
    ///
    /// 参数 `bean_name` 是注册名，`decl_warp_bean` 是声明式组件元数据；无容器实现
    /// 返回 `None`。对应 Java: `ContextAware#registerDeclWrapBean`。
    fn register_decl_wrap_bean(
        &self,
        _bean_name: &str,
        _decl_warp_bean: DeclWarpBean,
    ) -> Option<Bean> {
        None
    }
}
