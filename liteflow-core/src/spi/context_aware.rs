//! 对应 Java 类：com.yomahub.liteflow.spi.ContextAware
//!
//! 环境容器 SPI 接口。Java 版基于泛型 `<T> T getBean(String name)` 从
//! Spring/本地容器中取 Bean；Rust 无运行时泛型擦除容器，统一 Rust 化为
//! 基于 `Arc<dyn Any + Send + Sync>` 的 bean 容器，取用时由调用方 downcast。

use std::any::Any;
use std::sync::Arc;

use super::spi_priority::SpiPriority;

/// 任意类型 bean 的容器句柄（对应 Java 的 Object bean）
pub type Bean = Arc<dyn Any + Send + Sync>;

/// 对应 ContextAware（sync 版本，与 Java SPI 定义一致）
pub trait ContextAware: SpiPriority + Send + Sync {
    /// 对应 getBean(String name)
    fn get_bean(&self, name: &str) -> Option<Bean>;

    /// 对应 registerBean(String beanName, Object bean)，返回注册后的 bean
    fn register_bean(&self, name: &str, bean: Bean) -> Bean;

    /// 对应 hasBean(String beanName)
    fn has_bean(&self, name: &str) -> bool;

    /// 对应 registerOrGet(String beanName, Class<T> clazz)：
    /// 已存在则返回现有 bean，否则用 factory 构造并注册。
    /// （Java 通过 Class 反射实例化；Rust 侧以工厂闭包替代。）
    fn register_or_get(&self, name: &str, factory: &dyn Fn() -> Bean) -> Bean;
}
