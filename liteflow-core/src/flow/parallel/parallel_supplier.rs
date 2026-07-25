//! 对应 Java 类：com.yomahub.liteflow.flow.parallel.ParallelSupplier
//!
//! 并行异步 worker 对象，提供给并行执行使用。
//! Java 版是 implements Supplier<WhenFutureObj> 的类（持有 executableItem /
//! currChainName / slotIndex，get() 内执行并捕获异常封装为 WhenFutureObj）；
//! Rust 化为 async trait：get() -> WhenFutureObj，异常不抛出，封装进结果载体。

use crate::flow::parallel::when_future_obj::WhenFutureObj;
use async_trait::async_trait;

/// 并行异步 worker（对应 ParallelSupplier / Supplier<WhenFutureObj>）
#[async_trait]
pub trait ParallelSupplier: Send + Sync {
    /// 对应 Supplier.get()：执行并行项并返回结果载体
    async fn get(&self) -> WhenFutureObj;
}

/// 闭包即 ParallelSupplier（对应 Java 侧的匿名 Supplier 用法）
#[async_trait]
impl<F, Fut> ParallelSupplier for F
where
    F: Fn() -> Fut + Send + Sync,
    Fut: std::future::Future<Output = WhenFutureObj> + Send,
{
    async fn get(&self) -> WhenFutureObj {
        self().await
    }
}
