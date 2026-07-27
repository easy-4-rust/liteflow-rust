//! S2-A 挂接测试：flow.executor NodeExecutor 重试主干层 + flow.parallel 补齐。
//!
//! 覆盖：
//! - 重试第 N 次成功（首次 + 重试，总调用次数对齐 retry_count + 1 上限）
//! - ChainEnd（ChainEndException）不重试直接上抛
//! - 非 retry_for 异常不重试
//! - 重试次数耗尽上抛最后一次异常
//! - NodeExecutorHelper：默认执行器走单例缓存，组件自定义执行器被采用
//! - flow.parallel：WhenFutureObj 三态构造、complete_on_timeout 超时兜底

use liteflow_core::NodeComponent;
use liteflow_core::el::NodeRef;
use liteflow_core::exception::LiteflowError;
use liteflow_core::flow::element::node::Node;
use liteflow_core::flow::executor::{DefaultNodeExecutor, NodeExecutor, NodeExecutorHelper};
use liteflow_core::flow::parallel::{CompletableFutureTimeout, WhenFutureObj, complete_on_timeout};
use liteflow_core::slot::{CmpContext, Ctx, Frame, Slot};
use serde_json::Value;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

/// 可控制第几次调用才成功的组件，用于验证重试主干语义。
/// process 抛出的错误在 Node 层被包为 LiteflowError::NodeExec（ChainEnd 除外）。
struct FlakyCmp {
    calls: AtomicUsize,
    /// 第几次调用（1 起）开始成功；usize::MAX 表示永远失败
    succeed_on: usize,
    /// 对应 getRetryCount()
    retry_count: usize,
    /// 对应 getRetryForExceptions()：是否声明 NodeExec 为可重试异常
    retry_for_node_exec: bool,
    /// 是否固定抛 ChainEnd
    chain_end: bool,
}

/// `process` 成功但 `on_success` 失败的组件，用于验证 Java 写结果时序。
struct OnSuccessFailureCmp {
    calls: AtomicUsize,
}

#[async_trait::async_trait]
impl NodeComponent for OnSuccessFailureCmp {
    async fn process(&self, _ctx: &CmpContext) -> Result<Value, LiteflowError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(Value::Bool(true))
    }

    async fn on_success(&self, _ctx: &CmpContext) -> Result<(), LiteflowError> {
        Err(LiteflowError::Custom("on_success failed".to_string()))
    }
}

#[async_trait::async_trait]
impl NodeComponent for FlakyCmp {
    async fn process(&self, _ctx: &CmpContext) -> Result<Value, LiteflowError> {
        let n = self.calls.fetch_add(1, Ordering::SeqCst) + 1;
        if self.chain_end {
            return Err(LiteflowError::ChainEnd);
        }
        if n >= self.succeed_on {
            Ok(Value::from(n))
        } else {
            Err(LiteflowError::Custom(format!("boom #{n}")))
        }
    }

    fn retry_count(&self) -> usize {
        self.retry_count
    }

    fn is_retry_for(&self, e: &LiteflowError) -> bool {
        self.retry_for_node_exec && matches!(e, LiteflowError::NodeExec { .. })
    }
}

fn ctx_frame() -> (Ctx, Frame) {
    let slot = Arc::new(Slot::new("req-1".to_string(), "chain1", Value::Null));
    (Ctx::new(slot), Frame::root())
}

fn node_of(cmp: FlakyCmp) -> Node {
    Node::new(NodeRef::new("a"), Arc::new(cmp))
}

/// 验证 Java Node 元数据访问方法、bind 覆盖以及 clone 隔离语义。
#[test]
fn node_java_named_metadata_and_copy_are_isolated() {
    let cmp = FlakyCmp {
        calls: AtomicUsize::new(0),
        succeed_on: 1,
        retry_count: 0,
        retry_for_node_exec: false,
        chain_end: false,
    };
    let mut node = node_of(cmp);
    node.set_id("node-a");
    node.set_node_instance_id("node-a-0");
    node.set_tag("blue");
    node.set_name("订单校验");
    node.set_type(liteflow_core::NodeTypeEnum::Common);
    node.set_clazz("example.OrderCheckComponent");
    node.set_cmp_data(r#"{"strict":true}"#);
    node.set_curr_chain_id("order-chain");
    node.set_script("return true");
    node.set_language("qlexpress");
    node.set_compiled(true);
    node.put_bind_data("tenant", "a");
    node.put_bind_data("tenant", "b");

    assert_eq!(node.get_id(), "node-a");
    assert_eq!(node.get_node_instance_id(), Some("node-a-0"));
    assert_eq!(node.get_tag(), Some("blue"));
    assert_eq!(node.get_name(), "订单校验");
    assert_eq!(node.get_type(), Some(liteflow_core::NodeTypeEnum::Common));
    assert_eq!(node.get_clazz(), Some("example.OrderCheckComponent"));
    assert_eq!(node.get_cmp_data(), Some(r#"{"strict":true}"#));
    assert_eq!(node.get_curr_chain_id(), Some("order-chain"));
    assert_eq!(node.get_script(), Some("return true"));
    assert_eq!(node.get_language(), Some("qlexpress"));
    assert!(node.is_compiled());
    assert_eq!(
        node.get_execute_type(),
        liteflow_core::ExecuteableTypeEnum::Node
    );
    assert_eq!(node.get_bind_data("tenant"), Some("b"));

    let mut copied = node.copy();
    copied.put_bind_data("tenant", "copy");
    copied.set_tag("copy");
    assert_eq!(node.get_bind_data("tenant"), Some("b"));
    assert_eq!(node.get_tag(), Some("blue"));
    assert_eq!(copied.get_bind_data("tenant"), Some("copy"));
    assert_eq!(copied.get_tag(), Some("copy"));
}

/// 验证 Node 的 Java ThreadLocal 兼容状态在父子 Frame 间保持隔离。
#[test]
fn node_task_local_state_is_isolated_and_removable() {
    let cmp = FlakyCmp {
        calls: AtomicUsize::new(0),
        succeed_on: 1,
        retry_count: 0,
        retry_for_node_exec: false,
        chain_end: false,
    };
    let node = node_of(cmp);
    let (ctx, mut parent) = ctx_frame();

    node.set_access_result(&mut parent, true);
    node.set_is_continue_on_error_result(&mut parent, true);
    node.set_loop_index(&mut parent, 7, 2);
    node.set_curr_loop_object(&mut parent, 7, serde_json::json!({"id": "parent"}));
    node.set_step_data(&mut parent, serde_json::json!({"trace": "parent"}));
    node.set_is_end(&ctx, true);

    let mut child = parent.clone();
    node.set_access_result(&mut child, false);
    node.set_loop_index(&mut child, 7, 9);
    node.set_step_data(&mut child, serde_json::json!({"trace": "child"}));

    assert!(node.get_access_result(&parent));
    assert!(node.get_is_continue_on_error_result(&parent));
    assert_eq!(node.get_loop_index(&parent), Some(2));
    assert_eq!(
        node.get_curr_loop_object(&parent),
        Some(&serde_json::json!({"id": "parent"}))
    );
    assert_eq!(
        node.get_step_data(&parent),
        Some(serde_json::json!({"trace": "parent"}))
    );
    assert_eq!(node.get_loop_index(&child), Some(9));
    assert_eq!(
        node.get_step_data(&child),
        Some(serde_json::json!({"trace": "child"}))
    );
    assert!(node.get_is_end(&ctx));

    node.remove_access_result(&mut child);
    node.remove_is_continue_on_error_result(&mut child);
    node.remove_curr_loop_object(&mut child);
    node.remove_step_data(&mut child);
    node.remove_is_end(&ctx);

    assert!(!node.get_access_result(&child));
    assert!(!node.get_is_continue_on_error_result(&child));
    assert!(node.get_loop_index(&child).is_none());
    assert!(node.get_step_data(&child).is_none());
    assert!(!node.get_is_end(&ctx));
    assert!(node.get_access_result(&parent));
    assert_eq!(node.get_loop_index(&parent), Some(2));
}

/// 验证结果读取只访问单次执行缓存，父子任务写入互不覆盖。
#[tokio::test]
async fn node_item_result_cache_is_single_execution_and_task_isolated() {
    let cmp = FlakyCmp {
        calls: AtomicUsize::new(0),
        succeed_on: 1,
        retry_count: 0,
        retry_for_node_exec: false,
        chain_end: false,
    };
    let node = node_of(cmp);
    let (ctx, parent) = ctx_frame();

    assert!(matches!(
        node.execute(&ctx, &parent).await,
        Ok(Value::Number(number)) if number.as_u64() == Some(1)
    ));
    assert_eq!(
        node.get_item_result_meta_value(&parent),
        Some(Value::from(1))
    );
    assert_eq!(
        node.get_item_result_meta_value(&parent),
        Some(Value::from(1))
    );
    assert_eq!(
        ctx.inner.steps.lock().expect("步骤锁不应中毒").len(),
        1,
        "重复读取缓存不得再次执行组件"
    );

    let child = parent.clone();
    assert!(matches!(
        node.execute(&ctx, &child).await,
        Ok(Value::Number(number)) if number.as_u64() == Some(2)
    ));
    assert_eq!(
        node.get_item_result_meta_value(&child),
        Some(Value::from(2))
    );
    assert_eq!(
        node.get_item_result_meta_value(&parent),
        Some(Value::from(1)),
        "子任务新结果不得覆盖父任务快照"
    );

    node.remove_item_result_meta_value(&child);
    assert_eq!(node.get_item_result_meta_value(&child), None);
    assert_eq!(
        node.get_item_result_meta_value(&parent),
        Some(Value::from(1))
    );
}

/// 验证 Java 时序：process 产出结果后，即使 onSuccess 失败仍可读取结果。
#[tokio::test]
async fn node_item_result_is_cached_before_on_success() {
    let node = Node::new(
        NodeRef::new("boolean"),
        Arc::new(OnSuccessFailureCmp {
            calls: AtomicUsize::new(0),
        }),
    );
    let (ctx, frame) = ctx_frame();

    assert!(matches!(
        node.execute(&ctx, &frame).await,
        Err(LiteflowError::NodeExec { msg, .. }) if msg == "on_success failed"
    ));
    assert_eq!(
        node.get_item_result_meta_value(&frame),
        Some(Value::Bool(true))
    );
}

/// 重试第 3 次成功：retry_count=3，第 3 次调用才成功 → Ok，总调用 3 次
#[tokio::test]
async fn retry_succeeds_on_nth_attempt() {
    let cmp = FlakyCmp {
        calls: AtomicUsize::new(0),
        succeed_on: 3,
        retry_count: 3,
        retry_for_node_exec: true,
        chain_end: false,
    };
    let node = node_of(cmp);
    let (ctx, frame) = ctx_frame();
    let r = node.execute(&ctx, &frame).await;
    assert!(r.is_ok(), "should succeed on 3rd attempt: {r:?}");
    // Node 包装了组件实例，调用次数通过 CmpStep 间接验证：3 次执行 = 3 条 step
    let steps = ctx.inner.steps.lock().unwrap().len();
    assert_eq!(steps, 3, "1 first attempt + 2 retries = 3 executions");
}

/// ChainEnd（ChainEndException）不重试：retry_count 有富余也只执行 1 次并上抛
#[tokio::test]
async fn chain_end_is_not_retried() {
    let cmp = FlakyCmp {
        calls: AtomicUsize::new(0),
        succeed_on: usize::MAX,
        retry_count: 5,
        retry_for_node_exec: true,
        chain_end: true,
    };
    let node = node_of(cmp);
    let (ctx, frame) = ctx_frame();
    let r = node.execute(&ctx, &frame).await;
    assert!(matches!(r, Err(LiteflowError::ChainEnd)), "got: {r:?}");
    let steps = ctx.inner.steps.lock().unwrap().len();
    assert_eq!(steps, 1, "ChainEnd must not be retried");
}

/// 非 retry_for 声明范围的异常不重试：第 1 次失败即上抛
#[tokio::test]
async fn error_outside_retry_for_is_not_retried() {
    let cmp = FlakyCmp {
        calls: AtomicUsize::new(0),
        succeed_on: 2,
        retry_count: 5,
        retry_for_node_exec: false,
        chain_end: false,
    };
    let node = node_of(cmp);
    let (ctx, frame) = ctx_frame();
    let r = node.execute(&ctx, &frame).await;
    assert!(
        matches!(r, Err(LiteflowError::NodeExec { .. })),
        "got: {r:?}"
    );
    let steps = ctx.inner.steps.lock().unwrap().len();
    assert_eq!(steps, 1, "error outside retry_for must not be retried");
}

/// 重试次数耗尽上抛最后一次异常：retry_count=2 → 共 3 次执行后 Err
#[tokio::test]
async fn exhausted_retries_throw_last_error() {
    let cmp = FlakyCmp {
        calls: AtomicUsize::new(0),
        succeed_on: usize::MAX,
        retry_count: 2,
        retry_for_node_exec: true,
        chain_end: false,
    };
    let node = node_of(cmp);
    let (ctx, frame) = ctx_frame();
    let r = node.execute(&ctx, &frame).await;
    match r {
        Err(LiteflowError::NodeExec { msg, .. }) => {
            assert!(
                msg.contains("boom #3"),
                "last error must be thrown, got: {msg}"
            );
        }
        other => panic!("expected NodeExec, got: {other:?}"),
    }
    let steps = ctx.inner.steps.lock().unwrap().len();
    assert_eq!(steps, 3, "1 first attempt + 2 retries = 3 executions");
}

/// NodeExecutorHelper：组件未指定执行器时返回缓存的 DefaultNodeExecutor 单例
#[tokio::test]
async fn helper_caches_default_executor_singleton() {
    let helper = NodeExecutorHelper::load_instance();
    let e1 = helper.build_node_executor(None);
    let e2 = helper.build_node_executor(None);
    assert!(
        Arc::ptr_eq(&e1, &e2),
        "default executor must be cached singleton"
    );
    // 组件指定自定义执行器时直接采用该实例
    let custom: Arc<dyn NodeExecutor> = Arc::new(DefaultNodeExecutor);
    let e3 = helper.build_node_executor(Some(custom.clone()));
    assert!(Arc::ptr_eq(&e3, &custom));
}

/// flow.parallel：WhenFutureObj 三态构造（成功/失败/超时）
#[test]
fn when_future_obj_variants() {
    let ok = WhenFutureObj::success("a");
    assert!(ok.is_success() && !ok.is_timeout() && ok.get_ex().is_none());
    assert_eq!(ok.get_executor_id(), "a");

    let fail = WhenFutureObj::fail("b", LiteflowError::Custom("x".into()));
    assert!(!fail.is_success() && !fail.is_timeout() && fail.get_ex().is_some());

    let timeout = WhenFutureObj::time_out("c");
    assert!(!timeout.is_success() && timeout.is_timeout());
    assert!(matches!(timeout.get_ex(), Some(LiteflowError::WhenTimeout)));
}

/// flow.parallel：complete_on_timeout 在超时后兜底为默认值，及时完成时返回原结果
#[tokio::test]
async fn complete_on_timeout_semantics() {
    // 及时完成 → 原结果
    let v = complete_on_timeout(0, async { 42 }, Duration::from_secs(1)).await;
    assert_eq!(v, 42);
    // 超时 → 默认值
    let v = complete_on_timeout(
        7,
        async {
            tokio::time::sleep(Duration::from_secs(5)).await;
            42
        },
        Duration::from_millis(20),
    )
    .await;
    assert_eq!(v, 7);

    // timeoutAfter → checked exception 映射为 Result 错误。
    let timeout_error =
        CompletableFutureTimeout::timeout_after::<()>(Duration::from_millis(1)).await;
    assert!(matches!(timeout_error, Err(LiteflowError::WhenTimeout)));

    // Tokio 所有权语义：超时后原 Future 被 drop，不会遗留后台任务。
    struct DropProbe(Arc<AtomicUsize>);
    impl Drop for DropProbe {
        fn drop(&mut self) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }
    let dropped = Arc::new(AtomicUsize::new(0));
    let future_dropped = Arc::clone(&dropped);
    let value = CompletableFutureTimeout::complete_on_timeout(
        9,
        async move {
            let _probe = DropProbe(future_dropped);
            tokio::time::sleep(Duration::from_secs(5)).await;
            42
        },
        Duration::from_millis(1),
    )
    .await;
    assert_eq!(value, 9);
    assert_eq!(dropped.load(Ordering::SeqCst), 1);
}
