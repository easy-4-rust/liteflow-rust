//! Java 执行器层级选择规则。

use crate::enums::ConditionTypeEnum;
use crate::exception::{LFResult, LiteflowError};

use super::ExecutorCondition;

/// 根据 Condition、Chain 与全局配置构建执行器选择结果。
///
/// Java 通过向下转型读取 `LoopCondition` 或 `WhenCondition`；Rust 调用方直接传入
/// 已类型化的可选构建器名称，从而保留相同优先级并避免运行时强制类型转换。
///
/// 对应 Java:
/// `com.yomahub.liteflow.thread.ExecutorCondition.ExecutorConditionBuilder`。
pub struct ExecutorConditionBuilder;

impl ExecutorConditionBuilder {
    /// 构建执行器条件。
    ///
    /// 优先级严格保持 Java 语义：Condition > Chain > 全局。WHEN 在
    /// `when_thread_pool_isolate` 为 `true` 时，即使没有显式名称也使用全局构建器
    /// 创建 Condition 隔离实例。
    ///
    /// 对应 Java: `ExecutorConditionBuilder#buildExecutorCondition`。
    pub fn build_executor_condition(
        condition_executor_class: Option<&str>,
        chain_executor_class: Option<&str>,
        when_thread_pool_isolate: bool,
        global_executor_class: &str,
        condition_type: ConditionTypeEnum,
    ) -> LFResult<ExecutorCondition> {
        let condition_executor_class = non_blank(condition_executor_class);
        let chain_level = non_blank(chain_executor_class).is_some();

        match condition_type {
            ConditionTypeEnum::For | ConditionTypeEnum::While | ConditionTypeEnum::Iterator => {
                Ok(ExecutorCondition::create(
                    condition_executor_class.is_some(),
                    chain_level,
                    condition_executor_class.map(ToOwned::to_owned),
                ))
            }
            ConditionTypeEnum::When => {
                let condition_level =
                    when_thread_pool_isolate || condition_executor_class.is_some();
                let executor_class = condition_executor_class
                    .unwrap_or(global_executor_class)
                    .to_string();
                Ok(ExecutorCondition::create(
                    condition_level,
                    chain_level,
                    Some(executor_class),
                ))
            }
            unsupported => Err(LiteflowError::Custom(format!(
                "unsupported executor condition type: {unsupported:?}"
            ))),
        }
    }
}

fn non_blank(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}
