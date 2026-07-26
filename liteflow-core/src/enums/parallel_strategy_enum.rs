//! 对应 com.yomahub.liteflow.enums.ParallelStrategyEnum（2.11+）：
//! WHEN 并行完成策略，由 ParallelStrategyExecutor 四种执行器实现。

/// 并行策略枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ParallelStrategyEnum {
    /// 全部完成（AllOfParallelExecutor，默认）
    All,
    /// 任一成功即返回（AnyOfParallelExecutor，ANY(true)）
    Any,
    /// 按比例成功（PercentageOfParallelExecutor，PERCENTAGE(p)）
    Percentage,
    /// 指定分支成功（SpecifyParallelExecutor，MUST("id")）
    Specify,
}
