//! Node 构建前后的生命周期钩子。
//!
//! 对应 Java:
//! `com.yomahub.liteflow.lifecycle.PostProcessNodeBuildLifeCycle`。

use super::life_cycle::LifeCycle;
use crate::flow::element::node::Node;

/// 在可执行 Node 构建前后接收同一个 Node。
///
/// Rust 为 EL 中每个节点出现位置构造独立 Node，因此生命周期修改会保留在该
/// 出现位置的真实执行对象中。对应 Java:
/// `com.yomahub.liteflow.lifecycle.PostProcessNodeBuildLifeCycle`。
pub trait PostProcessNodeBuildLifeCycle: LifeCycle {
    /// 在 Node 完成基础组件与规则元数据装配、尚未分配实例编号前调用。
    ///
    /// 参数 `node` 可修改名称、标签和其他构建期元数据，修改会进入最终执行对象。
    /// 对应 Java: `PostProcessNodeBuildLifeCycle#postProcessBeforeNodeBuild`。
    fn post_process_before_node_build(&self, node: &mut Node);

    /// 在 Node 完成实例编号及全部构建期装配后调用。
    ///
    /// 参数 `node` 与 before 阶段为同一个 Node，可用于观测最终构建结果。
    /// 对应 Java: `PostProcessNodeBuildLifeCycle#postProcessAfterNodeBuild`。
    fn post_process_after_node_build(&self, node: &Node);
}
