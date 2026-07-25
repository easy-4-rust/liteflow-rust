//! 对应 WhenCondition：并行执行，委托给 ParallelStrategyExecutor。
//! 字段对齐 Java：ignoreError / any / percentage / specifyIdSet(must) /
//! maxWaitTime / threadExecutorClass。

use crate::enums::ParallelStrategyEnum;
use crate::exception::{LFResult, LiteflowError};
use crate::flow::element::executable::Executable;
use crate::flow::parallel::strategy::{
    all_of::AllOfParallelExecutor, any_of::AnyOfParallelExecutor,
    percentage_of::PercentageOfParallelExecutor, specify_of::SpecifyParallelExecutor,
    ParallelOpts, ParallelStrategyExecutor,
};
use crate::slot::{Ctx, Frame};
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

pub struct WhenCondition {
    executable_list: Vec<Arc<dyn Executable>>,
    pub ignore_error: bool,
    pub any: bool,
    pub percentage: Option<f64>,
    /// specifyIdSet（按节点 id / 别名匹配）
    pub must: Vec<String>,
    pub max_wait_ms: Option<u64>,
    /// threadExecutorClass（Rust 端记录在案，统一走 tokio 调度）
    pub thread_executor: Option<String>,
}

impl WhenCondition {
    pub fn new(executable_list: Vec<Arc<dyn Executable>>) -> Self {
        Self {
            executable_list,
            ignore_error: false,
            any: false,
            percentage: None,
            must: Vec::new(),
            max_wait_ms: None,
            thread_executor: None,
        }
    }

    fn strategy(&self) -> ParallelStrategyEnum {
        if !self.must.is_empty() {
            ParallelStrategyEnum::Specify
        } else if self.any {
            ParallelStrategyEnum::Any
        } else if self.percentage.is_some() {
            ParallelStrategyEnum::Percentage
        } else {
            ParallelStrategyEnum::All
        }
    }

    /// must id → 分支序号
    fn must_idx(&self) -> HashSet<usize> {
        self.executable_list
            .iter()
            .enumerate()
            .filter(|(_, e)| self.must.iter().any(|m| m == e.id()))
            .map(|(i, _)| i)
            .collect()
    }
}

#[async_trait]
impl Executable for WhenCondition {
    async fn execute(&self, ctx: &Ctx, frame: &Frame) -> LFResult<Value> {
        if self.executable_list.is_empty() {
            return Ok(Value::Null);
        }
        let opts = ParallelOpts {
            ignore_error: self.ignore_error,
            must_idx: self.must_idx(),
        };
        let executor: Box<dyn ParallelStrategyExecutor> = match self.strategy() {
            ParallelStrategyEnum::All => Box::new(AllOfParallelExecutor),
            ParallelStrategyEnum::Any => Box::new(AnyOfParallelExecutor),
            ParallelStrategyEnum::Percentage => Box::new(PercentageOfParallelExecutor {
                percentage: self.percentage.unwrap_or(1.0),
            }),
            ParallelStrategyEnum::Specify => Box::new(SpecifyParallelExecutor),
        };
        // 并行分支共享同一 slot（Java 同一 slotIndex）
        let items: Vec<Arc<dyn Executable>> = self.executable_list.clone();
        let fut = executor.execute(items, &opts, ctx.clone(), frame.clone());
        match self.max_wait_ms {
            Some(ms) => match tokio::time::timeout(Duration::from_millis(ms), fut).await {
                Ok(r) => r,
                Err(_) => Err(LiteflowError::WhenTimeout),
            },
            None => fut.await,
        }
    }

    fn id(&self) -> &str {
        "WHEN"
    }
}
