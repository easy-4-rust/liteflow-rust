//! Rust 闭包组件适配器。

use async_trait::async_trait;
use serde_json::Value;

use crate::exception::LiteflowError;
use crate::slot::CmpContext;

use super::NodeComponent;

/// 将异步闭包适配为 `NodeComponent`。
pub struct FnComponent<F>(pub F);

#[async_trait]
impl<F, Fut> NodeComponent for FnComponent<F>
where
    F: Fn(CmpContext) -> Fut + Send + Sync + 'static,
    Fut: std::future::Future<Output = Result<Value, LiteflowError>> + Send,
{
    async fn process(&self, ctx: &CmpContext) -> Result<Value, LiteflowError> {
        (self.0)(ctx.clone()).await
    }
}

/// 创建闭包组件。
pub fn cmp<F, Fut>(function: F) -> FnComponent<F>
where
    F: Fn(CmpContext) -> Fut + Send + Sync + 'static,
    Fut: std::future::Future<Output = Result<Value, LiteflowError>> + Send,
{
    FnComponent(function)
}
