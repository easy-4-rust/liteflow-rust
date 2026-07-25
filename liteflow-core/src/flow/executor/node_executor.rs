//! 对应 Java 类：com.yomahub.liteflow.flow.executor.NodeExecutor
//!
//! 节点执行器——自定义执行策略需要实现该 trait（Java 为抽象类，Rust 化为
//! 带默认实现的 async trait）。默认 execute() 为重试循环主干：
//! 首次执行 + 最多 retry_count 次重试；ChainEnd（对应 ChainEndException）
//! 不重试直接上抛；仅当异常命中组件声明的 retry_for 语义
//! （NodeComponent::is_retry_for，对应 getRetryForExceptions）才重试；
//! 次数用尽上抛最后一次异常。

use crate::exception::{LFResult, LiteflowError};
use crate::flow::element::node::Node;
use crate::slot::{Ctx, Frame};
use async_trait::async_trait;
use serde_json::Value;

/// 节点执行器（对应 NodeExecutor 抽象类）
#[async_trait]
pub trait NodeExecutor: Send + Sync + 'static {
    /// 对应 NodeExecutor#execute(NodeComponent)：执行器执行入口。
    /// 若需要更大维度的执行方式可以重写该方法。
    ///
    /// 循环体调用 Node::execute_once（对应 instance.execute() 的单次执行逻辑）。
    async fn execute(&self, node: &Node, ctx: &Ctx, frame: &Frame) -> LFResult<Value> {
        let retry_count = node.instance().retry_count();
        for i in 0..=retry_count {
            let result = if i == 0 {
                // 先执行一次
                node.execute_once(ctx, frame).await
            } else {
                // 进入重试逻辑
                self.retry(node, i, ctx, frame).await
            };
            match result {
                Ok(v) => return Ok(v),
                // ChainEndException 无需重试，直接上抛
                Err(LiteflowError::ChainEnd) => return Err(LiteflowError::ChainEnd),
                Err(e) => {
                    // 两种情况不重试：
                    // 1) 抛出的异常不在组件声明的 retry_for 范围内
                    // 2) 已经重试次数大于等于配置次数（上抛最后一次异常）
                    if !node.instance().is_retry_for(&e) || i >= retry_count {
                        return Err(e);
                    }
                }
            }
        }
        unreachable!("retry loop always returns within retry_count + 1 iterations")
    }

    /// 对应 NodeExecutor#retry(NodeComponent, currentRetryCount)：
    /// 执行重试逻辑——打印日志后再次执行单次业务逻辑。
    /// 子类（实现者）可通过重写该方法进行重试逻辑的控制。
    async fn retry(
        &self,
        node: &Node,
        current_retry_count: usize,
        ctx: &Ctx,
        frame: &Frame,
    ) -> LFResult<Value> {
        // 对齐 Java 日志：LOG.info("[{}]:component[{}] performs {} retry",
        //   requestId, displayName, currentRetryCount + 1)
        //（沿用 Java 的计数口径：第 1 次重试打印 "performs 2 retry"）
        println!(
            "[liteflow] [{}]:component[{}] performs {} retry",
            ctx.inner.request_id,
            node.display_name(),
            current_retry_count + 1
        );
        // 执行业务逻辑的主要入口（单次执行）
        node.execute_once(ctx, frame).await
    }
}
