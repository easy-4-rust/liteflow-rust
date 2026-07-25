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
use crate::slot::CmpContext;
use async_trait::async_trait;
use serde_json::Value;

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
