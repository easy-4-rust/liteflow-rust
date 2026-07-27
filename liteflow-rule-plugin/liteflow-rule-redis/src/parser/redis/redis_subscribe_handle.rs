//! Redis 订阅任务生命周期句柄。

use tokio::sync::watch;
use tokio::task::JoinHandle;

/// Redis 订阅任务的可停止句柄。
///
/// 该对象是 Rust 异步生命周期载体；Java `RedisParserSubscribeMode` 的 Redisson
/// listener 由客户端托管，Rust 通过显式句柄保证任务可等待停止。
pub struct RedisSubscribeHandle {
    stop_sender: watch::Sender<bool>,
    task: Option<JoinHandle<()>>,
}

impl RedisSubscribeHandle {
    pub(crate) fn new(stop_sender: watch::Sender<bool>, task: JoinHandle<()>) -> Self {
        Self {
            stop_sender,
            task: Some(task),
        }
    }

    /// 通知订阅任务停止，并等待底层 Pub/Sub 连接退出。
    pub async fn stop(mut self) {
        let _ = self.stop_sender.send(true);
        if let Some(task) = self.task.take() {
            let _ = task.await;
        }
    }

    /// 返回订阅任务是否已经结束。
    #[must_use]
    pub fn is_finished(&self) -> bool {
        self.task
            .as_ref()
            .is_none_or(tokio::task::JoinHandle::is_finished)
    }
}

impl Drop for RedisSubscribeHandle {
    fn drop(&mut self) {
        let _ = self.stop_sender.send(true);
    }
}
