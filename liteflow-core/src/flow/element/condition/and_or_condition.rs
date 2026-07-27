//! 对应 AndOrCondition：AND/OR 布尔短路。

use super::{Condition, expect_bool};
use crate::enums::ConditionTypeEnum;
use crate::exception::LFResult;
use crate::flow::element::condition::BooleanConditionTypeEnum;
use crate::flow::element::executable::Executable;
use crate::slot::{Ctx, Frame};
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;

pub struct AndOrCondition {
    condition_type: BooleanConditionTypeEnum,
    items: Vec<Arc<dyn Executable>>,
}

impl AndOrCondition {
    pub fn new(condition_type: BooleanConditionTypeEnum, items: Vec<Arc<dyn Executable>>) -> Self {
        Self {
            condition_type,
            items,
        }
    }
}

#[async_trait]
impl Executable for AndOrCondition {
    async fn execute(&self, ctx: &Ctx, frame: &Frame) -> LFResult<Value> {
        // 2.16 语义：先按 isAccess 过滤（不可访问/异常的子项被排除），
        // 再对剩余子项做 allMatch / anyMatch
        let mut accessible = Vec::with_capacity(self.items.len());
        for item in &self.items {
            if item.is_access(ctx, frame).await {
                accessible.push(item);
            }
        }
        match self.condition_type {
            BooleanConditionTypeEnum::And => {
                for item in accessible {
                    let v = item.execute(ctx, frame).await?;
                    if !expect_bool(item.id(), &v)? {
                        return Ok(Value::Bool(false));
                    }
                }
                Ok(Value::Bool(true))
            }
            BooleanConditionTypeEnum::Or => {
                for item in accessible {
                    let v = item.execute(ctx, frame).await?;
                    if expect_bool(item.id(), &v)? {
                        return Ok(Value::Bool(true));
                    }
                }
                Ok(Value::Bool(false))
            }
        }
    }

    fn id(&self) -> &str {
        match self.condition_type {
            BooleanConditionTypeEnum::And => "AND",
            BooleanConditionTypeEnum::Or => "OR",
        }
    }
}

impl Condition for AndOrCondition {
    fn condition_type(&self) -> ConditionTypeEnum {
        ConditionTypeEnum::AndOr
    }
}
