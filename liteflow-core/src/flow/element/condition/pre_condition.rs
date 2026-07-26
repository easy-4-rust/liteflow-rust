//! 对应 Java 类：com.yomahub.liteflow.flow.element.condition.PreCondition
//!
//! 前置 Condition。
//!
//! 差异说明：
//! - Java PreCondition 持有 executableList 并循环执行多个可执行项；Rust 端
//!   EL 中 PRE(...) 只包裹单个表达式，由 builder 保证单 item，故持单字段。

use super::Condition;
use crate::enums::ConditionTypeEnum;
use crate::exception::LFResult;
use crate::flow::element::executable::Executable;
use crate::slot::{Ctx, Frame};
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;

pub struct PreCondition {
    item: Arc<dyn Executable>,
}

impl PreCondition {
    pub fn new(item: Arc<dyn Executable>) -> Self {
        Self { item }
    }

    /// 对应 Java PreCondition#getConditionType
    pub fn condition_type(&self) -> ConditionTypeEnum {
        ConditionTypeEnum::Pre
    }
}

#[async_trait]
impl Executable for PreCondition {
    async fn execute(&self, ctx: &Ctx, frame: &Frame) -> LFResult<Value> {
        self.item.execute(ctx, frame).await
    }
    fn id(&self) -> &str {
        "PRE"
    }
    fn is_pre_or_finally(&self) -> bool {
        true
    }
}

impl Condition for PreCondition {
    fn condition_type(&self) -> ConditionTypeEnum {
        PreCondition::condition_type(self)
    }
}
