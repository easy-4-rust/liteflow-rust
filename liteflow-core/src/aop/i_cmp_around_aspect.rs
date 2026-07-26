//! 全局组件拦截器接口。
//!
//! 对应 Java: `com.yomahub.liteflow.aop.ICmpAroundAspect`。
//! Java 把可变 `NodeComponent` 作为回调参数；Rust 组件是共享 trait object，
//! 因而传入包含节点元数据、Slot 与请求上下文的 `CmpContext`，避免反射和可变全局状态。

use async_trait::async_trait;

use crate::exception::LiteflowError;
use crate::slot::CmpContext;

/// 全局组件拦截器协议。
///
/// 对应 Java: `com.yomahub.liteflow.aop.ICmpAroundAspect`。
#[async_trait]
pub trait ICmpAroundAspect: Send + Sync + 'static {
    /// 在组件自身 `before_process` 之前执行。
    ///
    /// 对应 Java: `ICmpAroundAspect#beforeProcess(NodeComponent)`。
    async fn before_process(&self, ctx: &CmpContext) {
        // 默认切面不注入前置行为。
        let _ = ctx;
    }
    /// 在组件成功完成 `process` 与 `on_success` 后执行。
    ///
    /// 对应 Java: `ICmpAroundAspect#onSuccess(NodeComponent)`。
    async fn on_success(&self, ctx: &CmpContext) {
        // 默认切面不注入成功回调。
        let _ = ctx;
    }
    /// 在组件自身 `after_process` 之后执行，无论成功或失败都调用。
    ///
    /// 对应 Java: `ICmpAroundAspect#afterProcess(NodeComponent)`。
    async fn after_process(&self, ctx: &CmpContext) {
        // 默认切面不注入后置行为。
        let _ = ctx;
    }
    /// 在组件执行失败后接收原始错误。
    ///
    /// 对应 Java: `ICmpAroundAspect#onError(NodeComponent, Exception)`。
    async fn on_error(&self, ctx: &CmpContext, error: &LiteflowError) {
        // 默认切面只保留错误传播，不改变错误对象。
        let _ = (ctx, error);
    }
}
