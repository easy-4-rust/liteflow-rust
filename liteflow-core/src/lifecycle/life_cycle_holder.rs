//! 对应 com.yomahub.liteflow.lifecycle.LifeCycleHolder（2.11+）：
//! 生命周期钩子的持有与注册中心。Java 以静态方法持有；
//! Rust 端由 FlowBus 持有实例（显式注册表语义）。

use super::LifeCycle;
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
    /// 根据生命周期对象的真实阶段类型登记实现。
    ///
    /// Java 使用 `isAssignableFrom` 依次识别五类子接口；Rust 由
    /// `LifeCycle::register_life_cycle` 做对象安全动态分派，并仍写入本对象的
    /// 五个强类型列表，不使用 `Any` 或字符串伪造类型判断。
    ///
    /// - `life_cycle`: 待登记的生命周期实现。
    ///
    /// 对应 Java: `LifeCycleHolder#addLifeCycle`。
    pub fn add_life_cycle(&mut self, life_cycle: Arc<dyn LifeCycle>) {
        life_cycle.register_life_cycle(self);
    }

    /// 返回脚本引擎初始化生命周期列表。
    ///
    /// 返回切片与当前持有器使用同一事实来源。对应 Java:
    /// `LifeCycleHolder#getPostProcessScriptEngineInitLifeCycleList`。
    #[must_use]
    pub fn get_post_process_script_engine_init_life_cycle_list(
        &self,
    ) -> &[Arc<dyn PostProcessScriptEngineInitLifeCycle>] {
        &self.script_engine_init
    }

    /// 返回链路构建完成生命周期列表。
    ///
    /// 对应 Java: `LifeCycleHolder#getPostProcessChainBuildLifeCycleList`。
    #[must_use]
    pub fn get_post_process_chain_build_life_cycle_list(
        &self,
    ) -> &[Arc<dyn PostProcessChainBuildLifeCycle>] {
        &self.chain_build
    }

    /// 返回节点构建完成生命周期列表。
    ///
    /// 对应 Java: `LifeCycleHolder#getPostProcessNodeBuildLifeCycleList`。
    #[must_use]
    pub fn get_post_process_node_build_life_cycle_list(
        &self,
    ) -> &[Arc<dyn PostProcessNodeBuildLifeCycle>] {
        &self.node_build
    }

    /// 返回流程执行前后生命周期列表。
    ///
    /// 对应 Java: `LifeCycleHolder#getPostProcessFlowExecuteLifeCycleList`。
    #[must_use]
    pub fn get_post_process_flow_execute_life_cycle_list(
        &self,
    ) -> &[Arc<dyn PostProcessFlowExecuteLifeCycle>] {
        &self.flow_execute
    }

    /// 返回链路执行前后生命周期列表。
    ///
    /// 对应 Java: `LifeCycleHolder#getPostProcessChainExecuteLifeCycleList`。
    #[must_use]
    pub fn get_post_process_chain_execute_life_cycle_list(
        &self,
    ) -> &[Arc<dyn PostProcessChainExecuteLifeCycle>] {
        &self.chain_execute
    }

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
