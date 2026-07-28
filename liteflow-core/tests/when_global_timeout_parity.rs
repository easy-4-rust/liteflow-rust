//! WHEN 未显式设置等待时间时的 Java 全局配置消费回归测试。

use liteflow_core::{FlowBus, LiteflowConfig, LiteflowConfigGetter, TimeUnit, cmp};
use serde_json::Value;
use std::time::Duration;

/// 验证 `ParallelStrategyExecutor#setWhenConditionParams` 的配置回退语义。
///
/// WHEN 本身不设置 MAX_WAIT，运行时应使用全局
/// `whenMaxWaitTime + whenMaxWaitTimeUnit`，并记录真实超时分支。
#[tokio::test]
async fn when_without_local_deadline_uses_global_wait_configuration() {
    LiteflowConfigGetter::clean();
    let mut liteflow_config = LiteflowConfig::default();
    liteflow_config.set_when_max_wait_time(40);
    liteflow_config.set_when_max_wait_time_unit(TimeUnit::Milliseconds);
    LiteflowConfigGetter::set_liteflow_config(liteflow_config);

    let bus = FlowBus::new();
    bus.register(
        "slow",
        cmp(|_| async {
            tokio::time::sleep(Duration::from_millis(200)).await;
            Ok(Value::Null)
        }),
    );
    bus.add_chain("global-timeout", "WHEN(slow)").unwrap();

    let response = bus.execute("global-timeout").await;
    LiteflowConfigGetter::clean();

    assert!(!response.is_success());
    assert!(
        response
            .message
            .contains("Timed out when executing the component[slow]")
    );
    assert_eq!(response.get_timeout_items(), ["slow"]);
}
