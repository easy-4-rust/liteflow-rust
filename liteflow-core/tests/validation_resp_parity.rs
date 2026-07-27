//! Java `ValidationResp` 失败原因访问器语义回归测试。

use liteflow_core::common::entity::ValidationResp;
use liteflow_core::exception::LiteflowError;

/// 验证 Java 命名 getter 读取构造器和 setter 共同维护的真实异常状态。
#[test]
fn java_named_cause_getter_reads_the_real_failure_state() {
    let mut response = ValidationResp::fail(LiteflowError::Custom("first".to_string()));
    assert_eq!(
        response.get_cause().map(ToString::to_string).as_deref(),
        Some("first")
    );

    response.set_cause(Some(LiteflowError::Custom("second".to_string())));
    assert_eq!(
        response.get_cause().map(ToString::to_string).as_deref(),
        Some("second")
    );

    response.set_cause(None);
    assert!(response.get_cause().is_none());
}
