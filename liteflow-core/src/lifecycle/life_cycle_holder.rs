//! 对应 com.yomahub.liteflow.lifecycle.LifeCycleHolder（2.11+）：
//! 生命周期钩子的持有与注册中心。Java 以静态方法持有；
//! Rust 端由 FlowBus 持有实例（显式注册表语义）。

use super::post_process_chain_build_life_cycle::PostProcessChainBuildLifeCycle;
use super::post_process_chain_execute_life_cycle::PostProcessChainExecuteLifeCycle;
use super::post_process_flow_execute_life_cycle::PostProcessFlowExecuteLifeCycle;
use super::post_process_node_build_life_cycle::PostProcessNodeBuildLifeCycle;
use std::sync::Arc;

/// 生命周期钩子集合（对应 LifeCycleHolder 的四类钩子容器）
#[derive(Default)]
pub struct LifeCycleHolder {
    /// 节点构建钩子
    pub node_build: Vec<Arc<dyn PostProcessNodeBuildLifeCycle>>,
    /// 链路构建钩子
    pub chain_build: Vec<Arc<dyn PostProcessChainBuildLifeCycle>>,
    /// 流程执行钩子
    pub flow_execute: Vec<Arc<dyn PostProcessFlowExecuteLifeCycle>>,
    /// 链路执行钩子
    pub chain_execute: Vec<Arc<dyn PostProcessChainExecuteLifeCycle>>,
}
