//! ZooKeeper 规则源与 Vernal 配置组合场景。

use liteflow_vernal::LiteflowConfig;

/// 校验 ZooKeeper 插件契约与 Vernal 启用配置可组合。
pub async fn run_case() -> bool {
    LiteflowConfig::new().enable && liteflow_testcase_el_zk::run_case().await
}
