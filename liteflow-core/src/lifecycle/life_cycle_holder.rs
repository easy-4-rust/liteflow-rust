//! 对应 com.yomahub.liteflow.lifecycle.LifeCycleHolder（2.11+）：
//! 生命周期钩子的持有与注册中心。Java 以静态方法持有；
//! Rust 端由 FlowBus 持有实例（显式注册表语义）。

use super::post_process_chain_build_life_cycle::PostProcessChainBuildLifeCycle;
use super::post_process_chain_execute_life_cycle::PostProcessChainExecuteLifeCycle;
use super::post_process_flow_execute_life_cycle::PostProcessFlowExecuteLifeCycle;
use super::post_process_node_build_life_cycle::PostProcessNodeBuildLifeCycle;
use super::post_process_script_engine_init_life_cycle::PostProcessScriptEngineInitLifeCycle;
use std::sync::Arc;

/// 生命周期钩子集合。
///
/// 对应 Java: `com.yomahub.liteflow.lifecycle.LifeCycleHolder`。Java 使用五个
/// 静态列表；Rust 由每个 `FlowBus` 持有独立实例，避免不同运行时相互污染。
#[derive(Default)]
pub struct LifeCycleHolder {
    /// 脚本执行器初始化钩子
    pub script_engine_init: Vec<Arc<dyn PostProcessScriptEngineInitLifeCycle>>,
    /// 节点构建钩子
    pub node_build: Vec<Arc<dyn PostProcessNodeBuildLifeCycle>>,
    /// 链路构建钩子
    pub chain_build: Vec<Arc<dyn PostProcessChainBuildLifeCycle>>,
    /// 流程执行钩子
    pub flow_execute: Vec<Arc<dyn PostProcessFlowExecuteLifeCycle>>,
    /// 链路执行钩子
    pub chain_execute: Vec<Arc<dyn PostProcessChainExecuteLifeCycle>>,
}

impl LifeCycleHolder {
    /// 清空全部五类生命周期实现。
    ///
    /// 对应 Java: `LifeCycleHolder#clean`。清理后已有的 `Arc` 实现仍可由外部
    /// 持有，但当前运行时不会再调用它们。
    pub fn clean(&mut self) {
        self.script_engine_init.clear();
        self.chain_build.clear();
        self.node_build.clear();
        self.flow_execute.clear();
        self.chain_execute.clear();
    }
}
