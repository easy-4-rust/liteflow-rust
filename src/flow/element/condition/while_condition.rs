//! 对应 WhileCondition：条件循环（含并行形态）。

use super::loop_condition::{handle_future_list, run_sequential, submit_iteration};
use super::expect_bool;
use crate::exception::LFResult;
use crate::flow::element::executable::Executable;
use crate::slot::{Ctx, Frame};
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;
use tokio::task::JoinSet;

pub struct WhileCondition {
    while_item: Arc<dyn Executable>,
    pub parallel: Option<usize>,
    do_executor: Arc<dyn Executable>,
    break_item: Option<Arc<dyn Executable>>,
}

impl WhileCondition {
    pub fn new(
        while_item: Arc<dyn Executable>,
        parallel: Option<usize>,
        do_executor: Arc<dyn Executable>,
        break_item: Option<Arc<dyn Executable>>,
    ) -> Self {
        Self { while_item, parallel, do_executor, break_item }
    }
}

#[async_trait]
impl Executable for WhileCondition {
    async fn execute(&self, ctx: &Ctx, frame: &Frame) -> LFResult<Value> {
        let mut index = 0usize;
        if self.parallel.is_some() {
            let mut set: JoinSet<LFResult<Value>> = JoinSet::new();
            loop {
                let f = frame.push(index, None);
                let v = self.while_item.execute(ctx, &f).await?;
                if !expect_bool(self.while_item.id(), &v)? {
                    break;
                }
                if !submit_iteration(&mut set, &self.do_executor, self.break_item.as_ref(), ctx, frame, index, None).await? {
                    break;
                }
                index += 1;
            }
            return handle_future_list(set).await;
        }

        loop {
            let f = frame.push(index, None);
            let v = self.while_item.execute(ctx, &f).await?;
            if !expect_bool(self.while_item.id(), &v)? {
                break;
            }
            if !run_sequential(&self.do_executor, self.break_item.as_ref(), ctx, frame, index, None).await? {
                break;
            }
            index += 1;
        }
        Ok(Value::Null)
    }

    fn id(&self) -> &str {
        "WHILE"
    }
}
