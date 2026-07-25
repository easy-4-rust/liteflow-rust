//! S2-A 挂接测试：flow.executor NodeExecutor 重试主干层 + flow.parallel 补齐。
//!
//! 覆盖：
//! - 重试第 N 次成功（首次 + 重试，总调用次数对齐 retry_count + 1 上限）
//! - ChainEnd（ChainEndException）不重试直接上抛
//! - 非 retry_for 异常不重试
//! - 重试次数耗尽上抛最后一次异常
//! - NodeExecutorHelper：默认执行器走单例缓存，组件自定义执行器被采用
//! - flow.parallel：WhenFutureObj 三态构造、complete_on_timeout 超时兜底

use liteflow_core::el::NodeRef;
use liteflow_core::exception::LiteflowError;
use liteflow_core::flow::element::executable::Executable;
use liteflow_core::flow::element::node::Node;
use liteflow_core::flow::executor::{DefaultNodeExecutor, NodeExecutor, NodeExecutorHelper};
use liteflow_core::flow::parallel::{complete_on_timeout, WhenFutureObj};
use liteflow_core::slot::{CmpContext, Ctx, Frame, Slot};
use liteflow_core::NodeComponent;
use serde_json::Value;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
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
    assert!(matches!(r, Err(LiteflowError::NodeExec { .. })), "got: {r:?}");
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
            assert!(msg.contains("boom #3"), "last error must be thrown, got: {msg}");
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
    assert!(Arc::ptr_eq(&e1, &e2), "default executor must be cached singleton");
    // 组件指定自定义执行器时直接采用该实例
    let custom: Arc<dyn NodeExecutor> = Arc::new(DefaultNodeExecutor);
    let e3 = helper.build_node_executor(Some(custom.clone()));
    assert!(Arc::ptr_eq(&e3, &custom));
}

/// flow.parallel：WhenFutureObj 三态构造（成功/失败/超时）
#[test]
fn when_future_obj_variants() {
    let ok = WhenFutureObj::success("a");
    assert!(ok.is_success() && !ok.is_timeout() && ok.ex.is_none());
    assert_eq!(ok.executor_name, "a");

    let fail = WhenFutureObj::fail("b", LiteflowError::Custom("x".into()));
    assert!(!fail.is_success() && !fail.is_timeout() && fail.ex.is_some());

    let timeout = WhenFutureObj::time_out("c");
    assert!(!timeout.is_success() && timeout.is_timeout());
    assert!(matches!(timeout.ex, Some(LiteflowError::WhenTimeout)));
}

/// flow.parallel：complete_on_timeout 在超时后兜底为默认值，及时完成时返回原结果
#[tokio::test]
async fn complete_on_timeout_semantics() {
    // 及时完成 → 原结果
    let v = complete_on_timeout(0, async { 42 }, Duration::from_secs(1)).await;
    assert_eq!(v, 42);
    // 超时 → 默认值
    let v = complete_on_timeout(7, async {
        tokio::time::sleep(Duration::from_secs(5)).await;
        42
    }, Duration::from_millis(20))
    .await;
    assert_eq!(v, 7);
}
