//! 对应 flow.element.condition 包：每种 Condition 一个对象。

pub mod condition_key;
pub mod then_condition;
pub mod when_condition;
pub mod if_condition;
pub mod switch_condition;
pub mod loop_condition;
pub mod for_condition;
pub mod while_condition;
pub mod iterator_condition;
pub mod catch_condition;
pub mod and_or_condition;
pub mod not_condition;
pub mod retry_condition;
pub mod timeout_condition;
pub mod ignore_error_condition;
pub mod chain_bind_wrapper_condition;
pub mod bind_wrapper_condition;
pub mod pre_condition;
pub mod finally_condition;

use crate::exception::{LFResult, LiteflowError};
use serde_json::Value;

/// 期望布尔结果的元素返回其他类型时报错（IfTypeErrorException 语义）
pub fn expect_bool(name: &str, v: &Value) -> LFResult<bool> {
    match v {
        Value::Bool(b) => Ok(*b),
        other => Err(LiteflowError::NodeTypeError {
            node: name.to_string(),
            expect: "boolean".into(),
            actual: other.to_string(),
        }),
    }
}

/// IfCondition / SwitchCondition 的目标不可为 pre/finally
pub fn check_not_pre_finally(target: &dyn crate::flow::element::Executable, name: &str) -> LFResult<()> {
    if target.is_pre_or_finally() {
        Err(LiteflowError::TargetCannotBePreOrFinally(name.to_string()))
    } else {
        Ok(())
    }
}
