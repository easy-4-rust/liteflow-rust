//! Chain 注册前后的构建生命周期钩子。
//!
//! 对应 Java:
//! `com.yomahub.liteflow.lifecycle.PostProcessChainBuildLifeCycle`。

use super::life_cycle::LifeCycle;
use crate::flow::element::chain::Chain;

/// 在完整 Chain 写入 `FlowBus` 前后接收同一个 Chain 对象。
///
/// 对应 Java:
/// `com.yomahub.liteflow.lifecycle.PostProcessChainBuildLifeCycle`。
pub trait PostProcessChainBuildLifeCycle: LifeCycle {
    /// 在 Chain 写入注册表前执行。
    ///
    /// 参数 `chain` 是已经完成 Condition 构建、但尚未替换注册表旧值的可变
    /// Chain；回调对元数据的修改会随新 Chain 一起注册。
    /// 对应 Java: `PostProcessChainBuildLifeCycle#postProcessBeforeChainBuild`。
    fn post_process_before_chain_build(&self, chain: &mut Chain);

    /// 在 Chain 写入注册表后执行。
    ///
    /// 参数 `chain` 与 before 阶段接收同一个 Chain 对象，此时新 Chain 已可从
    /// `FlowBus` 查询。对应 Java:
    /// `PostProcessChainBuildLifeCycle#postProcessAfterChainBuild`。
    fn post_process_after_chain_build(&self, chain: &Chain);
}
