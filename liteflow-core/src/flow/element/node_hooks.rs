//! Node 执行期横切钩子。

use std::sync::Arc;

use crate::aop::ICmpAroundAspect;
use crate::monitor::MonitorBus;

/// 节点级切面与监控快照，由构建器注入。
///
/// 这是 Rust 为避免 Node 反向持有 FlowBus 而抽出的伴随状态，承接 Java 全局
/// CmpAroundAspectHolder 与 MonitorBus 的执行期读取；其中 `monitor` 是
/// `NodeComponent#getMonitorBus/setMonitorBus` 的显式不可变映射，不对应独立
/// Java 对象。
#[derive(Clone, Default)]
pub struct NodeHooks {
    /// 全局组件切面。
    pub aspects: Vec<Arc<dyn ICmpAroundAspect>>,
    /// 可选监控总线。
    pub monitor: Option<Arc<MonitorBus>>,
}
