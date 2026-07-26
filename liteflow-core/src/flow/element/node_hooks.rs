//! Node 执行期横切钩子。

use std::sync::Arc;

use crate::aop::ICmpAroundAspect;
use crate::monitor::MonitorBus;

/// 节点级切面与监控快照，由构建器注入。
#[derive(Clone, Default)]
pub struct NodeHooks {
    /// 全局组件切面。
    pub aspects: Vec<Arc<dyn ICmpAroundAspect>>,
    /// 可选监控总线。
    pub monitor: Option<Arc<MonitorBus>>,
}
