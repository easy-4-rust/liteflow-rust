//! Java `thread/` 包的执行器选择、缓存与真实并发边界测试。

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use liteflow_core::builder::el::LiteFlowChainELBuilder;
use liteflow_core::{
    ConditionTypeEnum, ExecuteOption, ExecutorBuilder, ExecutorConditionBuilder, ExecutorHelper,
    ExecutorService, FlowBus, LFResult, cmp, parse_el,
};
use serde_json::Value;

/// 构建固定并发度执行器，替代 Java 测试中的自定义 ExecutorBuilder 类。
struct FixedExecutorBuilder {
    maximum_pool_size: usize,
}

impl ExecutorBuilder for FixedExecutorBuilder {
    fn build_executor(&self) -> Arc<ExecutorService> {
        self.build_common_executor(
            self.maximum_pool_size,
            self.maximum_pool_size,
            32,
            "test-thread-",
        )
    }
}

/// 记录并发峰值和完成次数。
#[derive(Default)]
struct ConcurrencyProbe {
    active: AtomicUsize,
    peak: AtomicUsize,
    completed: AtomicUsize,
}

impl ConcurrencyProbe {
    async fn run(&self) -> LFResult<Value> {
        let active = self.active.fetch_add(1, Ordering::SeqCst) + 1;
        self.peak.fetch_max(active, Ordering::SeqCst);
        tokio::time::sleep(Duration::from_millis(15)).await;
        self.active.fetch_sub(1, Ordering::SeqCst);
        self.completed.fetch_add(1, Ordering::SeqCst);
        Ok(Value::Null)
    }
}

#[test]
fn executor_condition_preserves_java_scope_precedence() {
    let global = "com.example.GlobalExecutor";
    let condition = ExecutorConditionBuilder::build_executor_condition(
        Some("com.example.ConditionExecutor"),
        Some("com.example.ChainExecutor"),
        false,
        global,
        ConditionTypeEnum::For,
    )
    .expect("FOR 应支持执行器选择");
    assert!(condition.is_condition_level());
    assert!(condition.is_chain_level());
    assert_eq!(
        condition.condition_executor_class(),
        Some("com.example.ConditionExecutor")
    );
    assert_eq!(
        condition.get_condition_executor_class(),
        condition.condition_executor_class()
    );

    let chain = ExecutorConditionBuilder::build_executor_condition(
        None,
        Some("com.example.ChainExecutor"),
        false,
        global,
        ConditionTypeEnum::Iterator,
    )
    .expect("ITERATOR 应支持 Chain 层级执行器");
    assert!(!chain.is_condition_level());
    assert!(chain.is_chain_level());

    let isolated_when = ExecutorConditionBuilder::build_executor_condition(
        None,
        None,
        true,
        global,
        ConditionTypeEnum::When,
    )
    .expect("WHEN 隔离配置应使用全局构建器创建独立实例");
    assert!(isolated_when.is_condition_level());
    assert_eq!(isolated_when.condition_executor_class(), Some(global));
}

#[test]
fn executor_helper_caches_by_condition_and_chain_scope() {
    const EXECUTOR_CLASS: &str = "test.executor.CacheScope";
    let helper = ExecutorHelper::load_instance();
    helper.register_executor_builder(
        EXECUTOR_CLASS,
        Arc::new(FixedExecutorBuilder {
            maximum_pool_size: 1,
        }),
    );

    let first = helper
        .build_executor_service(
            Some(EXECUTOR_CLASS),
            None,
            "condition-a",
            "chain-a",
            ConditionTypeEnum::When,
        )
        .expect("应构建 Condition 执行器");
    let same = helper
        .build_executor_service(
            Some(EXECUTOR_CLASS),
            None,
            "condition-a",
            "chain-a",
            ConditionTypeEnum::When,
        )
        .expect("相同 Condition 应命中缓存");
    let other = helper
        .build_executor_service(
            Some(EXECUTOR_CLASS),
            None,
            "condition-b",
            "chain-a",
            ConditionTypeEnum::When,
        )
        .expect("不同 Condition 应创建隔离执行器");

    assert!(Arc::ptr_eq(&first, &same));
    assert!(!Arc::ptr_eq(&first, &other));
    assert_eq!(first.maximum_pool_size(), 1);
    assert_eq!(first.queue_capacity(), 32);
}

#[tokio::test]
async fn executor_shutdown_waits_for_active_task_and_rejects_new_work() {
    let executor_service = Arc::new(ExecutorService::new(1, 1, 1, "shutdown-test-"));
    let running_service = executor_service.clone();
    let running = tokio::spawn(async move {
        running_service
            .execute(async {
                tokio::time::sleep(Duration::from_millis(25)).await;
                42
            })
            .await
    });

    tokio::time::timeout(Duration::from_millis(100), async {
        while executor_service.active_count() == 0 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("任务应进入活动状态");

    assert!(
        ExecutorHelper::load_instance()
            .shutdown_await_termination(&executor_service, Duration::from_millis(100))
            .await
    );
    assert_eq!(
        running
            .await
            .expect("活动任务不应 panic")
            .expect("活动任务应完成"),
        42
    );
    assert!(executor_service.is_shutdown());
    assert!(
        executor_service
            .execute(async { Value::Null })
            .await
            .is_err(),
        "shutdown 后必须拒绝新任务"
    );
}

#[tokio::test]
async fn when_and_parallel_loop_use_registered_bounded_executors() {
    const WHEN_EXECUTOR: &str = "test.executor.SerialWhen";
    const LOOP_EXECUTOR: &str = "test.executor.SerialLoop";
    const CHAIN_EXECUTOR: &str = "test.executor.SerialChain";
    let helper = ExecutorHelper::load_instance();
    for executor_class in [WHEN_EXECUTOR, LOOP_EXECUTOR, CHAIN_EXECUTOR] {
        helper.register_executor_builder(
            executor_class,
            Arc::new(FixedExecutorBuilder {
                maximum_pool_size: 1,
            }),
        );
    }

    let when_probe = Arc::new(ConcurrencyProbe::default());
    let bus = FlowBus::new();
    for node_id in ["a", "b"] {
        let probe = when_probe.clone();
        bus.register(
            node_id,
            cmp(move |_context| {
                let probe = probe.clone();
                async move { probe.run().await }
            }),
        );
    }
    bus.add_chain(
        "serial_when",
        &format!("WHEN(a,b).threadPool(\"{WHEN_EXECUTOR}\")"),
    )
    .expect("应构建显式执行器 WHEN");

    let response = bus.execute("serial_when").await;
    assert!(response.is_success(), "{}", response.message);
    assert_eq!(when_probe.completed.load(Ordering::SeqCst), 2);
    assert_eq!(
        when_probe.peak.load(Ordering::SeqCst),
        1,
        "单 worker 执行器必须串行化两个 WHEN 分支"
    );

    let loop_probe = Arc::new(ConcurrencyProbe::default());
    let probe = loop_probe.clone();
    bus.register(
        "loop_node",
        cmp(move |_context| {
            let probe = probe.clone();
            async move { probe.run().await }
        }),
    );
    bus.add_chain(
        "serial_loop",
        &format!("FOR(4).parallel(true).DO(loop_node).threadPool(\"{LOOP_EXECUTOR}\")"),
    )
    .expect("应构建显式执行器并行循环");

    let response = bus.execute("serial_loop").await;
    assert!(response.is_success(), "{}", response.message);
    assert_eq!(loop_probe.completed.load(Ordering::SeqCst), 4);
    assert_eq!(
        loop_probe.peak.load(Ordering::SeqCst),
        1,
        "单 worker 执行器必须限制并行循环体并发"
    );

    let chain_probe = Arc::new(ConcurrencyProbe::default());
    for node_id in ["chain_a", "chain_b"] {
        let probe = chain_probe.clone();
        bus.register(
            node_id,
            cmp(move |_context| {
                let probe = probe.clone();
                async move { probe.run().await }
            }),
        );
    }
    let mut chain = LiteFlowChainELBuilder::new(bus.clone())
        .build_chain(
            "chain_scoped_when",
            parse_el("WHEN(chain_a,chain_b)").expect("应解析 Chain 层级 WHEN"),
        )
        .expect("应构建 Chain 层级 WHEN");
    chain.set_thread_pool_executor_class(CHAIN_EXECUTOR);
    bus.add_built_chain(chain);

    let response = bus.execute("chain_scoped_when").await;
    assert!(response.is_success(), "{}", response.message);
    assert_eq!(chain_probe.completed.load(Ordering::SeqCst), 2);
    assert_eq!(
        chain_probe.peak.load(Ordering::SeqCst),
        1,
        "未指定 Condition 执行器时必须使用 Chain 层级执行器"
    );
}

#[tokio::test]
async fn flow_executor_future_uses_registered_main_executor() {
    const MAIN_EXECUTOR: &str = "test.executor.SerialMain";
    ExecutorHelper::load_instance().register_executor_builder(
        MAIN_EXECUTOR,
        Arc::new(FixedExecutorBuilder {
            maximum_pool_size: 1,
        }),
    );

    let probe = Arc::new(ConcurrencyProbe::default());
    let bus = FlowBus::new();
    let component_probe = probe.clone();
    bus.register(
        "future_node",
        cmp(move |_context| {
            let component_probe = component_probe.clone();
            async move { component_probe.run().await }
        }),
    );
    bus.add_chain("future_chain", "THEN(future_node)")
        .expect("应构建异步执行链");

    let executor = bus.executor();
    let first = executor
        .execute_future_with_executor(
            "future_chain",
            Value::Null,
            ExecuteOption::of(),
            Some(MAIN_EXECUTOR),
        )
        .expect("应提交第一个 Future");
    let second = executor
        .execute_future_with_executor(
            "future_chain",
            Value::Null,
            ExecuteOption::of(),
            Some(MAIN_EXECUTOR),
        )
        .expect("应提交第二个 Future");

    let first = first.await.expect("第一个 Tokio 任务不应 panic");
    let second = second.await.expect("第二个 Tokio 任务不应 panic");
    assert!(first.is_success(), "{}", first.message);
    assert!(second.is_success(), "{}", second.message);
    assert_eq!(probe.completed.load(Ordering::SeqCst), 2);
    assert_eq!(
        probe.peak.load(Ordering::SeqCst),
        1,
        "主执行器必须对 execute2Future 等价入口执行真实并发限制"
    );
}
