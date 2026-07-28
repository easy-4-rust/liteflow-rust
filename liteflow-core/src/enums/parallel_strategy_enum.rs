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

impl ParallelStrategyEnum {
    /// 返回 Java 并行策略类型字符串。
    ///
    /// 对应 Java: `ParallelStrategyEnum#getStrategyType`。
    #[must_use]
    pub fn get_strategy_type(self) -> &'static str {
        match self {
            Self::Any => "anyOf",
            Self::All => "allOf",
            Self::Specify => "must",
            Self::Percentage => "percentageOf",
        }
    }

    /// 返回并行策略的中文说明。
    ///
    /// 文本逐项保留 Java enum 构造参数。对应 Java:
    /// `ParallelStrategyEnum#getDescription`。
    #[must_use]
    pub fn get_description(self) -> &'static str {
        match self {
            Self::Any => "完成任一任务",
            Self::All => "完成全部任务",
            Self::Specify => "完成指定 ID 任务",
            Self::Percentage => "完整指定阈值任务",
        }
    }

    /// 返回对应并行执行器的稳定类型名称。
    ///
    /// Java 返回 `Class<? extends ParallelStrategyExecutor>`；Rust 不使用反射类对象，
    /// `ParallelStrategyHelper` 按本枚举直接构造对应策略，因此返回可用于配置和
    /// 诊断的 PascalCase 类型名。对应 Java: `ParallelStrategyEnum#getClazz`。
    #[must_use]
    pub fn get_clazz(self) -> &'static str {
        match self {
            Self::Any => "AnyOfParallelExecutor",
            Self::All => "AllOfParallelExecutor",
            Self::Specify => "SpecifyParallelExecutor",
            Self::Percentage => "PercentageOfParallelExecutor",
        }
    }
}
