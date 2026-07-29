//! ExecutorHelper 的 Java v2.16.0 对象级缓存、选择和关闭语义验收。

use std::sync::{Arc, Mutex};
use std::time::Duration;

use liteflow_core::{
    ConditionTypeEnum, ExecutorBuilder, ExecutorHelper, ExecutorService,
    LiteFlowDefaultGlobalExecutorBuilder, LiteFlowDefaultMainExecutorBuilder,
};

struct NamedExecutorBuilder {
    workers: usize,
}

static EXECUTOR_HELPER_TEST_LOCK: Mutex<()> = Mutex::new(());

impl ExecutorBuilder for NamedExecutorBuilder {
    fn build_executor(&self) -> Arc<ExecutorService> {
        Arc::new(ExecutorService::new(
            self.workers,
            self.workers,
            7,
            "executor-helper-parity",
        ))
    }
}

fn restore_defaults(helper: &ExecutorHelper) {
    helper.configure(
        LiteFlowDefaultGlobalExecutorBuilder::CLASS_NAME,
        LiteFlowDefaultMainExecutorBuilder::CLASS_NAME,
        64,
        512,
        64,
        false,
        true,
    );
}

/// 验证 configure 状态、默认/显式/Hash 重载和主执行器缓存共用真实注册表。
///
/// 对应 Java: `buildWhenExecutor*`、`buildMainExecutor*`。
#[test]
fn configuration_and_all_builder_overloads_share_one_cache() {
    let _guard = EXECUTOR_HELPER_TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    const GLOBAL: &str = "tests.ExecutorHelperGlobal";
    const MAIN: &str = "tests.ExecutorHelperMain";
    let helper = ExecutorHelper::load_instance();
    helper.register_executor_builder(GLOBAL, Arc::new(NamedExecutorBuilder { workers: 2 }));
    helper.register_executor_builder(MAIN, Arc::new(NamedExecutorBuilder { workers: 3 }));
    helper.configure(GLOBAL, MAIN, 0, 9, 0, true, false);

    assert_eq!(helper.global_thread_pool_size(), 1);
    assert_eq!(helper.global_thread_pool_queue_size(), 9);
    assert_eq!(helper.main_executor_works(), 1);
    assert!(!helper.is_enabled_virtual_threads());

    let default_when = helper.build_when_executor().unwrap();
    let blank_when = helper.build_when_executor_for(Some("  ")).unwrap();
    assert!(Arc::ptr_eq(&default_when, &blank_when));
    assert_eq!(default_when.maximum_pool_size(), 2);

    let isolated_a = helper
        .build_when_executor_with_hash(Some(GLOBAL), "condition-a")
        .unwrap();
    let isolated_a_again = helper
        .build_when_executor_with_hash(Some(GLOBAL), "condition-a")
        .unwrap();
    let blank_hash = helper
        .build_when_executor_with_hash(Some(GLOBAL), " ")
        .unwrap();
    assert!(Arc::ptr_eq(&isolated_a, &isolated_a_again));
    assert!(!Arc::ptr_eq(&isolated_a, &default_when));
    assert!(Arc::ptr_eq(&blank_hash, &default_when));

    let main = helper.build_main_executor(None).unwrap();
    let blank_main = helper.build_main_executor(Some(" ")).unwrap();
    assert!(Arc::ptr_eq(&main, &blank_main));
    assert_eq!(main.maximum_pool_size(), 3);
    assert!(helper.executor_service_count() >= 3);

    restore_defaults(helper);
    assert!(main.is_shutdown(), "配置替换必须关闭旧 Rust 执行器");
    assert!(helper.is_enabled_virtual_threads());
}

/// 验证 Condition > Chain > Global 优先级、缓存键及未知构建器错误。
///
/// 对应 Java: `ExecutorHelper#buildExecutorService`。
#[test]
fn executor_scope_priority_and_unknown_builder_match_java() {
    let _guard = EXECUTOR_HELPER_TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    const GLOBAL: &str = "tests.ExecutorScopeGlobal";
    const CHAIN: &str = "tests.ExecutorScopeChain";
    const CONDITION: &str = "tests.ExecutorScopeCondition";
    let helper = ExecutorHelper::load_instance();
    for (name, workers) in [(GLOBAL, 1), (CHAIN, 2), (CONDITION, 3)] {
        helper.register_executor_builder(name, Arc::new(NamedExecutorBuilder { workers }));
    }
    helper.configure(GLOBAL, "tests.ExecutorScopeMain", 4, 5, 6, false, true);

    let condition = helper
        .build_executor_service(
            Some(CONDITION),
            Some(CHAIN),
            "condition-key",
            "chain-key",
            ConditionTypeEnum::When,
        )
        .unwrap();
    let chain = helper
        .build_executor_service(
            None,
            Some(CHAIN),
            "condition-key",
            "chain-key",
            ConditionTypeEnum::For,
        )
        .unwrap();
    let global = helper
        .build_executor_service(
            None,
            None,
            "condition-key",
            "chain-key",
            ConditionTypeEnum::For,
        )
        .unwrap();
    assert_eq!(condition.maximum_pool_size(), 3);
    assert_eq!(chain.maximum_pool_size(), 2);
    assert_eq!(global.maximum_pool_size(), 1);

    assert!(
        helper
            .build_when_executor_for(Some("tests.UnknownExecutorBuilder"))
            .is_err()
    );
    restore_defaults(helper);
}

/// 验证 Java clear 只清缓存；显式 shutdown 重载才关闭执行器。
#[tokio::test]
async fn clear_does_not_shutdown_and_default_shutdown_uses_real_service() {
    let _guard = EXECUTOR_HELPER_TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    const EXECUTOR: &str = "tests.ExecutorClearContract";
    let helper = ExecutorHelper::load_instance();
    helper.register_executor_builder(EXECUTOR, Arc::new(NamedExecutorBuilder { workers: 1 }));
    let service = helper.build_when_executor_for(Some(EXECUTOR)).unwrap();

    helper.clear_executor_service_map();
    assert_eq!(helper.executor_service_count(), 0);
    assert!(!service.is_shutdown());
    assert_eq!(service.execute(async { 42 }).await.unwrap(), 42);

    assert!(helper.shutdown_await_termination_default(&service).await);
    assert!(service.is_shutdown());
    assert!(
        helper
            .shutdown_await_termination(
                &ExecutorService::new(1, 1, 0, "explicit-timeout"),
                Duration::from_millis(10),
            )
            .await
    );
}
