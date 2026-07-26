//! 不可执行的抽象链条件。

use super::Condition;
use crate::enums::ConditionTypeEnum;
use crate::exception::{LFResult, LiteflowError};
use crate::flow::element::Executable;
use crate::slot::{Ctx, Frame};
use async_trait::async_trait;
use serde_json::Value;

/// 标记含未实现变量、不能执行的抽象 Chain。
///
/// 对应 Java:
/// `com.yomahub.liteflow.flow.element.condition.AbstractCondition`。
#[derive(Debug, Clone)]
pub struct AbstractCondition {
    curr_chain_id: String,
}

impl AbstractCondition {
    /// 使用当前 Chain id 创建抽象条件。
    #[must_use]
    pub fn new(curr_chain_id: impl Into<String>) -> Self {
        Self {
            curr_chain_id: curr_chain_id.into(),
        }
    }

    /// 返回当前抽象条件所在的 Chain id。
    ///
    /// 对应 Java: `Condition#getCurrChainId`。
    #[must_use]
    pub fn curr_chain_id(&self) -> &str {
        &self.curr_chain_id
    }

    /// 更新当前抽象条件所在的 Chain id。
    ///
    /// 对应 Java: `Condition#setCurrChainId`。
    pub fn set_curr_chain_id(&mut self, curr_chain_id: impl Into<String>) {
        self.curr_chain_id = curr_chain_id.into();
    }
}

#[async_trait]
impl Executable for AbstractCondition {
    /// 拒绝执行含未实现变量的抽象链。
    ///
    /// 对应 Java: `AbstractCondition#executeCondition`。
    async fn execute(&self, _ctx: &Ctx, _frame: &Frame) -> LFResult<Value> {
        Err(LiteflowError::ChainNotImplemented(format!(
            "chain[{}] contains unimplemented variables, cannot be executed",
            self.curr_chain_id
        )))
    }
}

impl Condition for AbstractCondition {
    fn condition_type(&self) -> ConditionTypeEnum {
        ConditionTypeEnum::Abstract
    }
}
