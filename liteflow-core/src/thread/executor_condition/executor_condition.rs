//! Java 执行器层级判定结果。

/// 保存一次并行条件应使用 Condition、Chain 还是全局执行器的判定结果。
///
/// 对应 Java:
/// `com.yomahub.liteflow.thread.ExecutorCondition.ExecutorCondition`。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutorCondition {
    condition_level: bool,
    chain_level: bool,
    condition_executor_class: Option<String>,
}

impl ExecutorCondition {
    /// 创建执行器条件。
    ///
    /// `condition_level` 表示 Condition 显式指定或 WHEN 隔离配置生效；
    /// `chain_level` 表示当前 Chain 指定了执行器；`condition_executor_class`
    /// 保存 Condition 层级最终采用的构建器名称。
    /// 对应 Java: `ExecutorCondition#create`。
    #[must_use]
    pub fn create(
        condition_level: bool,
        chain_level: bool,
        condition_executor_class: Option<String>,
    ) -> Self {
        Self {
            condition_level,
            chain_level,
            condition_executor_class,
        }
    }

    /// 返回是否使用 Condition 层级执行器。
    ///
    /// 对应 Java: `ExecutorCondition#isConditionLevel`。
    #[must_use]
    pub fn is_condition_level(&self) -> bool {
        self.condition_level
    }

    /// 返回是否使用 Chain 层级执行器。
    ///
    /// 对应 Java: `ExecutorCondition#isChainLevel`。
    #[must_use]
    pub fn is_chain_level(&self) -> bool {
        self.chain_level
    }

    /// 返回 Condition 层级执行器构建器名称。
    ///
    /// 对应 Java: `ExecutorCondition#getConditionExecutorClass`。
    #[must_use]
    pub fn condition_executor_class(&self) -> Option<&str> {
        self.condition_executor_class.as_deref()
    }

    /// 返回 Condition 层级执行器构建器名称。
    ///
    /// 未指定 Condition 执行器时返回 `None`，对应 Java 的 `null`。
    /// 对应 Java: `ExecutorCondition#getConditionExecutorClass`。
    #[must_use]
    pub fn get_condition_executor_class(&self) -> Option<&str> {
        self.condition_executor_class()
    }
}
