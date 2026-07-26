//! 脚本组件构建函数类型。

use std::sync::Arc;

use crate::core::NodeComponent;
use crate::exception::LFResult;

use super::ScriptKind;

/// 脚本插件向工厂注册的组件构建函数。
pub type ScriptComponentBuilder = fn(&str, ScriptKind, &str) -> LFResult<Arc<dyn NodeComponent>>;
