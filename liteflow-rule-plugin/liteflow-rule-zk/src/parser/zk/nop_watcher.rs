//! ZooKeeper 连接所需的空 watcher。

/// 连接级事件由业务路径的持久递归 Watch 分别处理，本 watcher 只接收会话状态事件。
pub(crate) struct NopWatcher;

impl zookeeper::Watcher for NopWatcher {
    fn handle(&self, _event: zookeeper::WatchedEvent) {
        // 业务 znode 变更由 ZkParserHelper 安装的路径 watcher 处理。
    }
}
