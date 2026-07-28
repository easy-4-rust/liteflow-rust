//! 脚本组件构建函数类型。

use std::sync::Arc;

use crate::core::NodeComponent;
use crate::exception::LFResult;

use super::ScriptKind;

/// 脚本插件向工厂注册的组件构建函数。
///
/// 这是 Rust inventory/显式注册替代 Java SPI 与反射构造的函数指针类型，不对应
/// 独立 Java 对象。参数依次为节点 ID、脚本种类和源码，返回初始化后的组件。
pub type ScriptComponentBuilder = fn(&str, ScriptKind, &str) -> LFResult<Arc<dyn NodeComponent>>;
