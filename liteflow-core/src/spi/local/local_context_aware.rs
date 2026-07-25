//! 对应 Java 类：com.yomahub.liteflow.spi.local.LocalContextAware
//!
//! 非 Spring 环境容器实现。Java 版是无状态空实现（getBean 恒 null、
//! registerBean 仅反射 newInstance 不落库）；Rust 版按 S2-B 设计要求
//! 提供真正可用的本地 bean 容器：DashMap（对应 Java 体系中常见的
//! ConcurrentHashMap 版本地容器语义），保证 register_or_get 等 API 可用。

use dashmap::DashMap;

use crate::spi::context_aware::{Bean, ContextAware};
use crate::spi::spi_priority::SpiPriority;

/// 对应 LocalContextAware：基于 DashMap 的本地 bean 容器
#[derive(Default)]
pub struct LocalContextAware {
    beans: DashMap<String, Bean>,
}

impl LocalContextAware {
    pub fn new() -> Self {
        Self::default()
    }
}

impl ContextAware for LocalContextAware {
    /// 对应 getBean(String name)
    fn get_bean(&self, name: &str) -> Option<Bean> {
        self.beans.get(name).map(|b| b.clone())
    }

    /// 对应 registerBean(String beanName, Object bean)
    fn register_bean(&self, name: &str, bean: Bean) -> Bean {
        self.beans.insert(name.to_string(), bean.clone());
        bean
    }

    /// 对应 hasBean(String beanName)
    fn has_bean(&self, name: &str) -> bool {
        self.beans.contains_key(name)
    }

    /// 对应 registerOrGet(String beanName, Class<T> clazz)
    fn register_or_get(&self, name: &str, factory: &dyn Fn() -> Bean) -> Bean {
        self.beans
            .entry(name.to_string())
            .or_insert_with(factory)
            .clone()
    }
}

impl SpiPriority for LocalContextAware {
    /// 对应 priority()：本地实现默认优先级 2
    fn priority(&self) -> i32 {
        2
    }
}
