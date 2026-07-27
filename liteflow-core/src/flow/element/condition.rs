//! Condition 抽象基座及 `flow.element.condition` 子包入口。

pub mod abstract_condition;
pub mod and_or_condition;
pub mod bind_wrapper_condition;
pub mod boolean_condition_type_enum;
pub mod catch_condition;
pub mod chain_bind_wrapper_condition;
pub mod condition_key;
pub mod finally_condition;
pub mod for_condition;
pub mod if_condition;
pub mod ignore_error_condition;
pub mod iterator_condition;
pub mod loop_condition;
pub mod not_condition;
pub mod pre_condition;
pub mod retry_condition;
pub mod switch_condition;
pub mod then_condition;
pub mod timeout_condition;
pub mod when_condition;
pub mod while_condition;

pub use boolean_condition_type_enum::BooleanConditionTypeEnum;

use crate::enums::ConditionTypeEnum;
use crate::exception::{LFResult, LiteflowError};
use crate::flow::element::Executable;
use serde_json::Value;

/// 所有流程条件的统一抽象。
///
/// Java 抽象类中的执行入口已经由 `Executable::execute` 承担；各 Rust 条件使用
/// 带类型的字段代替 `Map<String, List<Executable>>`，Condition 级 bind 数据则由
/// `BindWrapperCondition` 和 `Frame` 的 bind 栈承载。本 trait 保留共同的类型识别
/// 与默认 id 规则，使具体条件能够通过 `dyn Condition` 做动态分派。
///
/// 对应 Java: `com.yomahub.liteflow.flow.element.Condition`。
pub trait Condition: Executable {
    /// 返回条件类型。对应 Java: `Condition#getConditionType`。
    fn condition_type(&self) -> ConditionTypeEnum;

    /// 返回显式 id；未提供时按 Java 规则生成 `condition-{type}`。
    ///
    /// 对应 Java: `Condition#getId`。
    fn condition_id(&self) -> String {
        let id = self.id();
        if id.trim().is_empty() {
            format!("condition-{}", self.condition_type().get_name())
        } else {
            id.to_string()
        }
    }

    /// 返回条件标签。对应 Java: `Condition#getTag`。
    fn condition_tag(&self) -> Option<&str> {
        self.tag()
    }
}

/// 期望布尔结果的元素返回其他类型时报错。
///
/// 对应 Java `IfTypeErrorException` / `SwitchTypeErrorException` 的类型校验。
pub fn expect_bool(name: &str, value: &Value) -> LFResult<bool> {
    match value {
        Value::Bool(result) => Ok(*result),
        other => Err(LiteflowError::NodeTypeError {
            node: name.to_string(),
            expect: "boolean".into(),
            actual: other.to_string(),
        }),
    }
}

/// 校验 IF / SWITCH 的目标不是 PRE 或 FINALLY。
pub fn check_not_pre_finally(target: &dyn Executable, name: &str) -> LFResult<()> {
    if target.is_pre_or_finally() {
        Err(LiteflowError::TargetCannotBePreOrFinally(name.to_string()))
    } else {
        Ok(())
    }
}
