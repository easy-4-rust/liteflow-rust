//! 对应 core.NodeComponent 及其子类（NodeBooleanComponent / NodeSwitchComponent /
//! NodeForComponent / NodeIteratorComponent / NodeBreakComponent）。
//!
//! Rust 版合并为一个 trait，用返回值区分类型语义：
//! - 普通组件 → `Value::Null`
//! - 布尔组件（IF/WHILE/BREAK/AND/OR/NOT）→ `Value::Bool`
//! - SWITCH 组件 → `Value::String`（目标 id，可带 "id:tag"）
//! - FOR 组件 → 数字
//! - ITERATOR 组件 → `Value::Array`

use crate::exception::LiteflowError;
use crate::flow::executor::NodeExecutor;
use crate::slot::CmpContext;
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;

#[async_trait]
pub trait NodeComponent: Send + Sync + 'static {
    /// process() / processIf() / processSwitch() / processFor() / processIterator()
    async fn process(&self, ctx: &CmpContext) -> Result<Value, LiteflowError>;

    /// beforeProcess()
    async fn before_process(&self, _ctx: &CmpContext) -> Result<(), LiteflowError> {
        Ok(())
    }
    /// afterProcess()
    async fn after_process(&self, _ctx: &CmpContext) {}
    /// onError()
    async fn on_error(&self, _ctx: &CmpContext, _e: &LiteflowError) {}
    /// isAccess()
    fn is_access(&self, _ctx: &CmpContext) -> bool {
        true
    }
    /// isContinueOnError()
    fn is_continue_on_error(&self) -> bool {
        false
    }
    /// Rollbackable.rollback()
    async fn rollback(&self, _ctx: &CmpContext) {}
    /// getName()
    fn name(&self) -> &str {
        ""
    }
    /// getRetryCount()：最大重试次数（默认 0 = 不重试，总尝试次数 = retry_count + 1）
    fn retry_count(&self) -> usize {
        0
    }
    /// getRetryForExceptions() 语义：判断抛出的异常是否命中组件声明的可重试异常范围
    /// （Java 用 retryForExceptions 列表 + isAssignableFrom 判定，Rust 化为谓词方法）
    fn is_retry_for(&self, _e: &LiteflowError) -> bool {
        false
    }
    /// getNodeExecutorClass()：指定自定义节点执行器；None 表示使用 DefaultNodeExecutor
    /// （Java 返回 Class 由 NodeExecutorHelper 经 DI 容器实例化并缓存，
    /// Rust 端无 DI 容器，直接提供 Arc 实例）
    fn node_executor(&self) -> Option<Arc<dyn NodeExecutor>> {
        None
    }
}

/// 闭包即组件（按值接收上下文，CmpContext 为廉价 Clone 句柄）
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

pub fn cmp<F, Fut>(f: F) -> FnComponent<F>
where
    F: Fn(CmpContext) -> Fut + Send + Sync + 'static,
    Fut: std::future::Future<Output = Result<Value, LiteflowError>> + Send,
{
    FnComponent(f)
}
