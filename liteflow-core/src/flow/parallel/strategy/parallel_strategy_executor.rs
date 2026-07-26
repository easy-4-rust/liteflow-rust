//! 对应 Java: `ParallelStrategyExecutor`。

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;

use crate::exception::LFResult;
use crate::flow::element::Executable;
use crate::slot::{Ctx, Frame};

use super::ParallelOpts;

/// 并行执行策略统一接口。
#[async_trait]
pub trait ParallelStrategyExecutor: Send + Sync {
    /// 按策略执行全部分支并返回结算值。
    async fn execute(
        &self,
        items: Vec<Arc<dyn Executable>>,
        opts: &ParallelOpts,
        ctx: Ctx,
        frame: Frame,
    ) -> LFResult<Value>;
}
