//! 环境容器中的任意 Bean 句柄。

use std::any::Any;
use std::sync::Arc;

/// 任意类型 Bean 的线程安全句柄，对应 Java `Object` Bean。
pub type Bean = Arc<dyn Any + Send + Sync>;
