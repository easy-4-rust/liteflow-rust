//! 不可执行的抽象链条件。

use super::{Condition, ConditionBase};
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
    base: ConditionBase,
}

impl AbstractCondition {
    /// 使用当前 Chain id 创建抽象条件。
    #[must_use]
    pub fn new(curr_chain_id: impl Into<String>) -> Self {
        Self {
            base: ConditionBase::with_curr_chain_id(curr_chain_id),
        }
    }

    /// 返回当前抽象条件所在的 Chain id。
    ///
    /// 对应 Java: `Condition#getCurrChainId`。
    #[must_use]
    pub fn curr_chain_id(&self) -> &str {
        self.base.curr_chain_id.as_deref().unwrap_or_default()
    }

    /// 更新当前抽象条件所在的 Chain id。
    ///
    /// 对应 Java: `Condition#setCurrChainId`。
    pub fn set_curr_chain_id(&mut self, curr_chain_id: impl Into<String>) {
        <Self as Condition>::set_curr_chain_id(self, curr_chain_id);
    }

    /// 拒绝执行仍含未实现变量的抽象链。
    ///
    /// 参数 `ctx` 与 `frame` 对应 Java `slotIndex` 定位的执行状态；返回
    /// `ChainNotImplemented`，错误文本包含当前 Chain id。
    /// 对应 Java: `AbstractCondition#executeCondition`。
    pub async fn execute_condition(&self, ctx: &Ctx, frame: &Frame) -> LFResult<Value> {
        super::execute_condition_with_lifecycle(self, ctx, frame, async {
            Err(LiteflowError::ChainNotImplemented(format!(
                "chain[{}] contains unimplemented variables, cannot be executed",
                self.curr_chain_id()
            )))
        })
        .await
    }

    /// 返回抽象条件类型。
    ///
    /// 返回值固定为 `ConditionTypeEnum::Abstract`。
    /// 对应 Java: `AbstractCondition#getConditionType`。
    #[must_use]
    pub fn get_condition_type(&self) -> ConditionTypeEnum {
        ConditionTypeEnum::Abstract
    }
}

#[async_trait]
impl Executable for AbstractCondition {
    /// 拒绝执行含未实现变量的抽象链。
    ///
    /// 对应 Java: `AbstractCondition#executeCondition`。
    async fn execute(&self, ctx: &Ctx, frame: &Frame) -> LFResult<Value> {
        AbstractCondition::execute_condition(self, ctx, frame).await
    }

    fn collect_node_ids(&self) -> Vec<String> {
        Condition::get_all_node_in_condition(self)
    }
}

impl Condition for AbstractCondition {
    fn condition_base(&self) -> &ConditionBase {
        &self.base
    }

    fn condition_base_mut(&mut self) -> &mut ConditionBase {
        &mut self.base
    }

    fn condition_type(&self) -> ConditionTypeEnum {
        self.get_condition_type()
    }
}
