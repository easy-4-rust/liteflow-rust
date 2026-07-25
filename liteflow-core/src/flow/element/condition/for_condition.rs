//! 对应 ForCondition：计数循环、DO/BREAK、PARALLEL 并行。

use super::loop_condition::{handle_future_list, run_sequential, submit_iteration};
use crate::exception::{LFResult, LiteflowError};
use crate::flow::element::executable::Executable;
use crate::slot::{Ctx, Frame};
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;
use tokio::task::JoinSet;

pub struct ForCondition {
    for_node: Arc<dyn Executable>,
    pub parallel: Option<usize>,
    do_executor: Arc<dyn Executable>,
    break_item: Option<Arc<dyn Executable>>,
}

impl ForCondition {
    pub fn new(
        for_node: Arc<dyn Executable>,
        parallel: Option<usize>,
        do_executor: Arc<dyn Executable>,
        break_item: Option<Arc<dyn Executable>>,
    ) -> Self {
        Self { for_node, parallel, do_executor, break_item }
    }
}

#[async_trait]
impl Executable for ForCondition {
    async fn execute(&self, ctx: &Ctx, frame: &Frame) -> LFResult<Value> {
        let v = self.for_node.execute(ctx, frame).await?;
        let count = match &v {
            Value::Number(n) => n.as_u64().unwrap_or(0) as usize,
            Value::String(s) => {
                s.parse::<usize>().map_err(|_| LiteflowError::NodeTypeError {
                    node: self.for_node.id().to_string(),
                    expect: "number".into(),
                    actual: v.to_string(),
                })?
            }
            other => {
                return Err(LiteflowError::NodeTypeError {
                    node: self.for_node.id().to_string(),
                    expect: "number".into(),
                    actual: other.to_string(),
                })
            }
        };

        if self.parallel.is_some() {
            let mut set: JoinSet<LFResult<Value>> = JoinSet::new();
            for i in 0..count {
                if !submit_iteration(&mut set, &self.do_executor, self.break_item.as_ref(), ctx, frame, i, None).await? {
                    break;
                }
            }
            return handle_future_list(set).await;
        }

        for i in 0..count {
            if !run_sequential(&self.do_executor, self.break_item.as_ref(), ctx, frame, i, None).await? {
                break;
            }
        }
        Ok(Value::Null)
    }

    fn id(&self) -> &str {
        "FOR"
    }
}
