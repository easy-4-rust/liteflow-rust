//! Vernal 规则初始化状态机。

use liteflow_core::parser::RuleDefinitionPlan;

/// 区分尚未读取、已收集待按链构建、全部就绪和失败四种状态。
///
/// 该状态只由 `LiteflowRuntime` 初始化锁保护，不向业务代码暴露。
/// 对应 Java: `FlowBus.needInit()` 与 `PARSE_ONE_ON_FIRST_EXEC` 的 Chain 缓存状态。
pub(crate) enum RuleInitializationState {
    /// 尚未读取规则，供 `PARSE_ALL_ON_FIRST_EXEC` 使用。
    Uninitialized,
    /// 已读取格式定义，等待按执行链物化。
    Planned(RuleDefinitionPlan),
    /// 全部规则已经物化。
    Initialized,
    /// 首次初始化已失败；后续执行稳定返回同一根因。
    Failed(String),
}
