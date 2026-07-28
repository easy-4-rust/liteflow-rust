//! Rust 闭包组件适配器。

use async_trait::async_trait;
use serde_json::Value;

use crate::exception::LiteflowError;
use crate::slot::CmpContext;

use super::NodeComponent;

/// 将异步闭包适配为 NodeComponent 的 Rust 专用伴随类型。
///
/// 它承接 Java 匿名 NodeComponent/测试组件的职责，不对应独立 Java 类。
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
///
/// 参数 `function` 接收拥有型 CmpContext 并返回异步组件结果；返回值实现
/// NodeComponent，可直接注册到 FlowBus。
pub fn cmp<F, Fut>(function: F) -> FnComponent<F>
where
    F: Fn(CmpContext) -> Fut + Send + Sync + 'static,
    Fut: std::future::Future<Output = Result<Value, LiteflowError>> + Send,
{
    FnComponent(function)
}
