use std::sync::{Arc, Mutex};
use std::time::Duration;

use futures::Future;
use futures::future::BoxFuture;
use tokio::time::timeout;

use crate::flow::error::FlowResult;

/// 带超时控制的 Future 包装器
///
/// 为指定的 Future 添加超时机制，防止无限期阻塞
pub struct CompletableFutureTimeout {
    /// 内部的 Future，包含实际的业务逻辑
    inner: Arc<Mutex<Option<BoxFuture<'static, FlowResult<()>>>>>,
    /// 超时时间（秒）
    timeout_seconds: u64,
    /// 节点 ID
    node_id: String,
}

impl CompletableFutureTimeout {
    /// 创建新的带超时控制的 Future
    pub fn new(
        future: BoxFuture<'static, FlowResult<()>>,
        timeout_seconds: u64,
        node_id: String,
    ) -> Self {
        Self {
            inner: Arc::new(Mutex::new(Some(future))),
            timeout_seconds,
            node_id,
        }
    }

    /// 执行 Future，如果超时则返回超时错误
    pub async fn execute(&self) -> FlowResult<()> {
        let future = {
            let mut guard = self.inner.lock().unwrap();
            guard
                .take()
                .ok_or_else(|| anyhow::anyhow!("Future already consumed"))?
        };

        match timeout(Duration::from_secs(self.timeout_seconds), future).await {
            Ok(result) => result,
            Err(_) => Err(anyhow::anyhow!(
                "Node '{}' execution timeout after {} seconds",
                self.node_id,
                self.timeout_seconds
            )
            .into()),
        }
    }

    /// 检查 Future 是否已经被消费
    pub fn is_completed(&self) -> bool {
        self.inner.lock().unwrap().is_none()
    }
}

impl Future for CompletableFutureTimeout {
    type Output = FlowResult<()>;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let future = {
            let mut guard = self.inner.lock().unwrap();
            guard.take()
        };

        match future {
            Some(f) => {
                let mut pinned = Box::pin(f);
                match pinned.as_mut().poll(cx) {
                    std::task::Poll::Ready(result) => std::task::Poll::Ready(result),
                    std::task::Poll::Pending => {
                        // 将 future 放回，以便下次继续 poll
                        let mut guard = self.inner.lock().unwrap();
                        *guard = Some(pinned);
                        std::task::Poll::Pending
                    }
                }
            }
            None => std::task::Poll::Ready(Err(anyhow::anyhow!("Future already consumed").into())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::future::ready;

    #[tokio::test]
    async fn test_completable_future_timeout_success() {
        let future = Box::pin(ready(Ok(())));
        let timeout_future = CompletableFutureTimeout::new(future, 5, "test_node".to_string());
        let result = timeout_future.execute().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_completable_future_timeout_error() {
        let future = Box::pin(ready(Err(anyhow::anyhow!("Test error").into())));
        let timeout_future = CompletableFutureTimeout::new(future, 5, "test_node".to_string());
        let result = timeout_future.execute().await;
        assert!(result.is_err());
    }
}
