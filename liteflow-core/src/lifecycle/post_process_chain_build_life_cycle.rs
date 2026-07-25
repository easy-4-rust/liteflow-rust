//! 对应 com.yomahub.liteflow.lifecycle.PostProcessChainBuildLifeCycle（2.11+）：
//! 链路构建完成后的生命周期钩子（postProcessAfterChainBuild）。

/// 链路构建生命周期钩子
pub trait PostProcessChainBuildLifeCycle: Send + Sync + 'static {
    /// postProcessAfterChainBuild(chainId)
    fn post_process_after_chain_build(&self, chain_id: &str);
}
