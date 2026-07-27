//! ZooKeeper 规则插件离线契约场景。

use liteflow_rule_zk::ZkParserVO;

/// 构建 ZooKeeper 规则源而不访问外部服务。
pub async fn run_case() -> bool {
    let config = ZkParserVO::new("zk.example.test:2181", "/liteflow/flow");
    config.connect_str() == "zk.example.test:2181"
        && config.chain_path() == "/liteflow/flow"
        && config.validate().is_ok()
}
