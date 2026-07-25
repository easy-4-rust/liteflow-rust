//! 对应 Java 类：com.yomahub.liteflow.flow.element.condition.FinallyCondition
//!
//! 后置 Condition。
//!
//! 差异说明：
//! - Java FinallyCondition 持有 executableList 并循环执行多个可执行项；Rust 端
//!   EL 中 FINALLY(...) 只包裹单个表达式，由 builder 保证单 item，故持单字段。

use crate::enums::ConditionTypeEnum;
use crate::exception::LFResult;
use crate::flow::element::executable::Executable;
use crate::slot::{Ctx, Frame};
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;

pub struct FinallyCondition {
    item: Arc<dyn Executable>,
}

impl FinallyCondition {
    pub fn new(item: Arc<dyn Executable>) -> Self {
        Self { item }
    }

    /// 对应 Java FinallyCondition#getConditionType
    pub fn condition_type(&self) -> ConditionTypeEnum {
        ConditionTypeEnum::Finally
    }
}

#[async_trait]
impl Executable for FinallyCondition {
    async fn execute(&self, ctx: &Ctx, frame: &Frame) -> LFResult<Value> {
        self.item.execute(ctx, frame).await
    }
    fn id(&self) -> &str {
        "FINALLY"
    }
    fn is_pre_or_finally(&self) -> bool {
        true
    }
}
