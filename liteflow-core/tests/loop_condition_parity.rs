//! `LoopCondition` 公共状态与并行 Supplier 真实执行测试。

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use liteflow_core::exception::LFResult;
use liteflow_core::flow::element::condition::for_condition::ForCondition;
use liteflow_core::flow::element::executable::Executable;
use liteflow_core::slot::{Ctx, Frame, Slot};
use liteflow_core::{ExecuteableTypeEnum, LoopCondition};
use serde_json::Value;

struct Probe {
    id: &'static str,
    output: Value,
    calls: Arc<AtomicUsize>,
}

#[async_trait::async_trait]
impl Executable for Probe {
    async fn execute(&self, _ctx: &Ctx, _frame: &Frame) -> LFResult<Value> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(self.output.clone())
    }

    fn execute_type(&self) -> ExecuteableTypeEnum {
        ExecuteableTypeEnum::Node
    }

    fn id(&self) -> &str {
        self.id
    }
}

fn probe(id: &'static str, output: Value, calls: &Arc<AtomicUsize>) -> Arc<dyn Executable> {
    Arc::new(Probe {
        id,
        output,
        calls: Arc::clone(calls),
    })
}

#[tokio::test]
async fn loop_condition_java_state_drives_parallel_body_and_break_execution() {
    let old_calls = Arc::new(AtomicUsize::new(0));
    let body_calls = Arc::new(AtomicUsize::new(0));
    let break_calls = Arc::new(AtomicUsize::new(0));
    let mut condition =
        ForCondition::with_count(4, None, probe("old-body", Value::Null, &old_calls), None);

    condition.set_do_executor(probe("body", Value::Null, &body_calls));
    condition.set_break_item(probe("break", Value::Bool(true), &break_calls));
    condition.set_thread_pool_executor_class(
        "com.yomahub.liteflow.thread.LiteFlowDefaultGlobalExecutorBuilder",
    );
    condition.set_parallel(true);

    assert_eq!(condition.get_do_executor().id(), "body");
    assert_eq!(
        condition.get_break_item().expect("BREAK 项应已写入").id(),
        "break"
    );
    assert_eq!(
        condition.get_thread_pool_executor_class(),
        Some("com.yomahub.liteflow.thread.LiteFlowDefaultGlobalExecutorBuilder")
    );
    assert!(condition.is_parallel());

    let ctx = Ctx::new(Arc::new(Slot::new(
        "loop-condition".to_string(),
        "loop-chain",
        Value::Null,
    )));
    condition
        .execute_condition(&ctx, &Frame::root())
        .await
        .expect("并行 FOR 应完成已提交任务并由 BREAK 停止后续提交");

    // BREAK 在提交第一轮后立即判断，因此只启动一次真实循环体。
    assert_eq!(old_calls.load(Ordering::SeqCst), 0);
    assert_eq!(body_calls.load(Ordering::SeqCst), 1);
    assert_eq!(break_calls.load(Ordering::SeqCst), 1);

    condition.set_parallel(false);
    assert!(!condition.is_parallel());
}
