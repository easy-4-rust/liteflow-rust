//! Java WhileOperator 匿名布尔组件的 Rust 等价实现。

use async_trait::async_trait;
use serde_json::Value;

use crate::exception::LFResult;
use crate::flow::element::Executable;
use crate::slot::{Ctx, Frame};

/// 返回固定布尔结果的条件节点。
pub(crate) struct BooleanLiteralCondition {
    value: bool,
}

impl BooleanLiteralCondition {
    /// 创建固定布尔结果条件。
    pub(crate) fn new(value: bool) -> Self {
        Self { value }
    }
}

#[async_trait]
impl Executable for BooleanLiteralCondition {
    async fn execute(&self, _ctx: &Ctx, _frame: &Frame) -> LFResult<Value> {
        Ok(Value::Bool(self.value))
    }

    fn id(&self) -> &str {
        if self.value {
            "LOOP_true"
        } else {
            "LOOP_false"
        }
    }
}
