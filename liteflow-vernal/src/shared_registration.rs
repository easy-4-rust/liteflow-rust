//! Vernal 组件注册动作的共享类型。

use std::sync::Arc;

use liteflow_core::{FlowBus, LFResult};

/// 可跨线程共享的 LiteFlow 组件注册动作。
///
/// 对应 Java: `ComponentScanner` 扫描完成后提交给容器的单个组件注册操作。
pub(crate) type SharedRegistration = Arc<dyn Fn(&FlowBus) -> LFResult<()> + Send + Sync + 'static>;
