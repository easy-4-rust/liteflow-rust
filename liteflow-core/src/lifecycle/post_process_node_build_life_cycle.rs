//! 对应 com.yomahub.liteflow.lifecycle.PostProcessNodeBuildLifeCycle（2.11+）：
//! 节点构建完成后的生命周期钩子（postProcessAfterNodeBuild）。

use super::life_cycle::LifeCycle;

/// 节点构建生命周期钩子
pub trait PostProcessNodeBuildLifeCycle: LifeCycle {
    /// postProcessAfterNodeBuild(nodeId)
    fn post_process_after_node_build(&self, node_id: &str);
}
