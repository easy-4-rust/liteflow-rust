//! ZooKeeper 连接所需的空 watcher。

/// 轮询模式下无需 SDK 回调；变更检测由 `RuleSourceWatcher` 统一承担。
pub(crate) struct NopWatcher;

impl zookeeper::Watcher for NopWatcher {
    fn handle(&self, _event: zookeeper::WatchedEvent) {
        // 轮询刷新由 RuleSourceWatcher 负责，SDK 回调事件有意不参与状态变更。
    }
}
