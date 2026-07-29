//! EL 包装操作符按 QLExpress 源码顺序执行的运行时差分测试。

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use liteflow_core::{FlowBus, LiteflowError, cmp};
use serde_json::Value;

/// 首次调用超过 timeout、第二次立即成功，用于区分 Timeout/Retry 的嵌套方向。
fn register_slow_once_component(bus: &FlowBus, attempts: Arc<AtomicUsize>) {
    bus.register(
        "slow_once",
        cmp(move |_| {
            let attempts = Arc::clone(&attempts);
            async move {
                let attempt = attempts.fetch_add(1, Ordering::SeqCst);
                if attempt == 0 {
                    tokio::time::sleep(Duration::from_millis(40)).await;
                }
                Ok(Value::Null)
            }
        }),
    );
}

/// 验证 Java QLExpress 的链式调用次序没有被 Rust Mods 字段合并重排。
///
/// 对应 Java:
/// `MaxWaitTimeOperator#build`、`RetryOperator#build`。
#[tokio::test]
async fn max_wait_and_retry_order_changes_real_execution_semantics() {
    // maxWait 在前：先生成 TimeoutCondition，再由 RetryCondition 包在外层。
    // 第一次内部超时后，外层 retry 应执行第二次并成功。
    let timeout_then_retry_bus = FlowBus::new();
    let timeout_then_retry_attempts = Arc::new(AtomicUsize::new(0));
    register_slow_once_component(
        &timeout_then_retry_bus,
        Arc::clone(&timeout_then_retry_attempts),
    );
    timeout_then_retry_bus
        .add_chain(
            "timeout_then_retry",
            "slow_once.maxWaitMilliseconds(10).retry(1)",
        )
        .expect("Java 合法 EL 应完成构建");

    let timeout_then_retry_response = timeout_then_retry_bus.execute("timeout_then_retry").await;
    assert!(
        timeout_then_retry_response.is_success(),
        "{}",
        timeout_then_retry_response.message
    );
    assert_eq!(
        timeout_then_retry_attempts.load(Ordering::SeqCst),
        2,
        "外层 RetryCondition 应在第一次 TimeoutCondition 超时后再执行一次"
    );

    // retry 在前：先生成 RetryCondition，再由 TimeoutCondition 包在外层。
    // 外层超时会取消整个内部重试过程，因此只有第一次组件调用。
    let retry_then_timeout_bus = FlowBus::new();
    let retry_then_timeout_attempts = Arc::new(AtomicUsize::new(0));
    register_slow_once_component(
        &retry_then_timeout_bus,
        Arc::clone(&retry_then_timeout_attempts),
    );
    retry_then_timeout_bus
        .add_chain(
            "retry_then_timeout",
            "slow_once.retry(1).maxWaitMilliseconds(10)",
        )
        .expect("反向 Java 合法 EL 应完成构建");

    let retry_then_timeout_response = retry_then_timeout_bus.execute("retry_then_timeout").await;
    assert!(!retry_then_timeout_response.is_success());
    assert!(
        retry_then_timeout_response.message.contains("timeout"),
        "{}",
        retry_then_timeout_response.message
    );
    assert_eq!(
        retry_then_timeout_attempts.load(Ordering::SeqCst),
        1,
        "外层 TimeoutCondition 应在内部 RetryCondition 获得错误前取消整次执行"
    );
}

/// 验证属性操作符不会改变 Java Condition 的动态类型或阻断后续扩展函数。
///
/// 对应 Java:
/// `IdOperator`、`TagOperator`、`IgnoreErrorOperator`、`DoOperator`、
/// `ElseOperator`。
#[tokio::test]
async fn property_modifiers_preserve_typed_condition_execution_paths() {
    let bus = FlowBus::new();
    let parallel_successes = Arc::new(AtomicUsize::new(0));
    let loop_calls = Arc::new(AtomicUsize::new(0));
    let left_calls = Arc::new(AtomicUsize::new(0));
    let right_calls = Arc::new(AtomicUsize::new(0));
    let catch_handler_calls = Arc::new(AtomicUsize::new(0));

    bus.register(
        "parallel_fail",
        cmp(|_| async { Err(LiteflowError::Custom("parallel failed".to_string())) }),
    );
    let parallel_success_counter = Arc::clone(&parallel_successes);
    bus.register(
        "parallel_ok",
        cmp(move |_| {
            let parallel_success_counter = Arc::clone(&parallel_success_counter);
            async move {
                parallel_success_counter.fetch_add(1, Ordering::SeqCst);
                Ok(Value::Null)
            }
        }),
    );

    let loop_counter = Arc::clone(&loop_calls);
    bus.register(
        "loop_body",
        cmp(move |_| {
            let loop_counter = Arc::clone(&loop_counter);
            async move {
                loop_counter.fetch_add(1, Ordering::SeqCst);
                Ok(Value::Null)
            }
        }),
    );

    bus.register("false_condition", cmp(|_| async { Ok(Value::Bool(false)) }));
    let left_counter = Arc::clone(&left_calls);
    bus.register(
        "left_branch",
        cmp(move |_| {
            let left_counter = Arc::clone(&left_counter);
            async move {
                left_counter.fetch_add(1, Ordering::SeqCst);
                Ok(Value::Null)
            }
        }),
    );
    let right_counter = Arc::clone(&right_calls);
    bus.register(
        "right_branch",
        cmp(move |_| {
            let right_counter = Arc::clone(&right_counter);
            async move {
                right_counter.fetch_add(1, Ordering::SeqCst);
                Ok(Value::Null)
            }
        }),
    );

    bus.register(
        "caught_failure",
        cmp(|_| async { Err(LiteflowError::Custom("caught failure".to_string())) }),
    );
    let catch_handler_counter = Arc::clone(&catch_handler_calls);
    bus.register(
        "catch_handler",
        cmp(move |_| {
            let catch_handler_counter = Arc::clone(&catch_handler_counter);
            async move {
                catch_handler_counter.fetch_add(1, Ordering::SeqCst);
                Ok(Value::Null)
            }
        }),
    );

    bus.add_chain(
        "property_when",
        r#"WHEN(parallel_fail,parallel_ok).id("parallel-id").tag("parallel-tag").ignoreError(true)"#,
    )
    .expect("带属性的 WhenCondition 仍应接受 ignoreError");
    bus.add_chain(
        "property_loop",
        r#"FOR(2).id("loop-id").tag("loop-tag").DO(loop_body)"#,
    )
    .expect("带属性的 LoopCondition 仍应接受 DO");
    bus.add_chain(
        "property_if",
        r#"IF(false_condition,left_branch).id("if-id").tag("if-tag").ELSE(right_branch)"#,
    )
    .expect("带属性的 IfCondition 仍应接受 ELSE");
    bus.add_chain(
        "property_catch",
        r#"CATCH(caught_failure).id("catch-id").tag("catch-tag").DO(catch_handler)"#,
    )
    .expect("带属性的 CatchCondition 仍应接受 DO");

    let when_response = bus.execute("property_when").await;
    assert!(when_response.is_success(), "{}", when_response.message);
    assert_eq!(parallel_successes.load(Ordering::SeqCst), 1);

    let loop_response = bus.execute("property_loop").await;
    assert!(loop_response.is_success(), "{}", loop_response.message);
    assert_eq!(loop_calls.load(Ordering::SeqCst), 2);

    let if_response = bus.execute("property_if").await;
    assert!(if_response.is_success(), "{}", if_response.message);
    assert_eq!(left_calls.load(Ordering::SeqCst), 0);
    assert_eq!(right_calls.load(Ordering::SeqCst), 1);

    let catch_response = bus.execute("property_catch").await;
    assert!(catch_response.is_success(), "{}", catch_response.message);
    assert_eq!(catch_handler_calls.load(Ordering::SeqCst), 1);
}
