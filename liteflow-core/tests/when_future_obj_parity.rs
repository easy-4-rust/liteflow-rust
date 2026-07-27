//! Java `WhenFutureObj` 三态构造和字段访问语义回归测试。

use liteflow_core::LiteflowError;
use liteflow_core::flow::parallel::WhenFutureObj;

#[test]
fn java_named_accessors_mutate_the_same_parallel_result_state() {
    let success = WhenFutureObj::success("node-a");
    assert!(success.is_success());
    assert!(!success.is_timeout());
    assert_eq!(success.get_executor_id(), "node-a");
    assert!(success.get_ex().is_none());

    let mut failure = WhenFutureObj::fail("node-b", LiteflowError::WhenExecute("boom".to_string()));
    assert!(!failure.is_success());
    assert!(!failure.is_timeout());
    assert_eq!(failure.get_executor_id(), "node-b");
    assert!(matches!(
        failure.get_ex(),
        Some(LiteflowError::WhenExecute(message)) if message == "boom"
    ));

    failure.set_success(true);
    failure.set_timeout(true);
    failure.set_executor_id("node-b-renamed");
    failure.set_ex(None);
    assert!(failure.is_success());
    assert!(failure.is_timeout());
    assert_eq!(failure.get_executor_id(), "node-b-renamed");
    assert_eq!(failure.executor_name(), "node-b-renamed");
    assert!(failure.get_ex().is_none());

    let timeout = WhenFutureObj::time_out("node-c");
    assert!(!timeout.is_success());
    assert!(timeout.is_timeout());
    assert_eq!(timeout.get_executor_id(), "node-c");
    assert!(matches!(
        timeout.get_ex(),
        Some(LiteflowError::WhenTimeout(message))
            if message == "Timed out when executing the component[node-c]"
    ));
}
