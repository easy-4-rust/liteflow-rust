//! 对应 com.yomahub.liteflow.lifecycle.PostProcessNodeBuildLifeCycle（2.11+）：
//! 节点构建完成后的生命周期钩子（postProcessAfterNodeBuild）。

/// 节点构建生命周期钩子
pub trait PostProcessNodeBuildLifeCycle: Send + Sync + 'static {
    /// postProcessAfterNodeBuild(nodeId)
    fn post_process_after_node_build(&self, node_id: &str);
}
