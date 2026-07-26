//! Redis 规则源与 Vernal 配置组合场景。

use liteflow_vernal::LiteflowConfig;

/// 校验 Redis 插件契约与 Vernal 启用配置可组合。
pub async fn run_case() -> bool {
    LiteflowConfig::new().enable && liteflow_testcase_el_redis::run_case().await
}
