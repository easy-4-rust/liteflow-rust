//! 对应 IteratorCondition：迭代循环，loopObject 传递（含并行形态）。

use super::loop_condition::{handle_future_list, run_sequential, submit_iteration};
use crate::exception::{LFResult, LiteflowError};
use crate::flow::element::executable::Executable;
use crate::slot::{Ctx, Frame};
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;
use tokio::task::JoinSet;

pub struct IteratorCondition {
    iterator_node: Arc<dyn Executable>,
    pub parallel: Option<usize>,
    do_executor: Arc<dyn Executable>,
    break_item: Option<Arc<dyn Executable>>,
}

impl IteratorCondition {
    pub fn new(
        iterator_node: Arc<dyn Executable>,
        parallel: Option<usize>,
        do_executor: Arc<dyn Executable>,
        break_item: Option<Arc<dyn Executable>>,
    ) -> Self {
        Self { iterator_node, parallel, do_executor, break_item }
    }
}

#[async_trait]
impl Executable for IteratorCondition {
    async fn execute(&self, ctx: &Ctx, frame: &Frame) -> LFResult<Value> {
        let v = self.iterator_node.execute(ctx, frame).await?;
        let list = match &v {
            Value::Array(a) => a.clone(),
            other => {
                return Err(LiteflowError::NodeTypeError {
                    node: self.iterator_node.id().to_string(),
                    expect: "array".into(),
                    actual: other.to_string(),
                })
            }
        };

        if self.parallel.is_some() {
            let mut set: JoinSet<LFResult<Value>> = JoinSet::new();
            for (i, obj) in list.iter().enumerate() {
                if !submit_iteration(&mut set, &self.do_executor, self.break_item.as_ref(), ctx, frame, i, Some(obj.clone())).await? {
                    break;
                }
            }
            return handle_future_list(set).await;
        }

        for (i, obj) in list.into_iter().enumerate() {
            if !run_sequential(&self.do_executor, self.break_item.as_ref(), ctx, frame, i, Some(obj)).await? {
                break;
            }
        }
        Ok(Value::Null)
    }

    fn id(&self) -> &str {
        "ITERATOR"
    }
}
