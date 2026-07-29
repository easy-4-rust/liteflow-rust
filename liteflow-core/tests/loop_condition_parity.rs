//! `LoopCondition` 公共状态与并行 Supplier 真实执行测试。

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use liteflow_core::exception::{LFResult, LiteflowError};
use liteflow_core::flow::element::condition::Condition;
use liteflow_core::flow::element::condition::for_condition::ForCondition;
use liteflow_core::flow::element::condition::iterator_condition::IteratorCondition;
use liteflow_core::flow::element::condition::pre_condition::PreCondition;
use liteflow_core::flow::element::condition::switch_condition::SwitchCondition;
use liteflow_core::flow::element::condition::while_condition::WhileCondition;
use liteflow_core::flow::element::executable::Executable;
use liteflow_core::slot::{Ctx, Frame, Slot};
use liteflow_core::{ExecuteableTypeEnum, LoopCondition};
use serde_json::Value;

struct Probe {
    id: &'static str,
    tag: Option<&'static str>,
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

    fn tag(&self) -> Option<&str> {
        self.tag
    }
}

fn probe(id: &'static str, output: Value, calls: &Arc<AtomicUsize>) -> Arc<dyn Executable> {
    Arc::new(Probe {
        id,
        tag: None,
        output,
        calls: Arc::clone(calls),
    })
}

fn tagged_probe(
    id: &'static str,
    tag: &'static str,
    output: Value,
    calls: &Arc<AtomicUsize>,
) -> Arc<dyn Executable> {
    Arc::new(Probe {
        id,
        tag: Some(tag),
        output,
        calls: Arc::clone(calls),
    })
}

struct SequenceProbe {
    id: &'static str,
    outputs: Vec<Value>,
    calls: Arc<AtomicUsize>,
}

struct DeniedProbe {
    id: &'static str,
    calls: Arc<AtomicUsize>,
}

#[async_trait::async_trait]
impl Executable for DeniedProbe {
    async fn execute(&self, _ctx: &Ctx, _frame: &Frame) -> LFResult<Value> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(Value::Number(3.into()))
    }

    async fn is_access(&self, _ctx: &Ctx, _frame: &Frame) -> bool {
        false
    }

    fn execute_type(&self) -> ExecuteableTypeEnum {
        ExecuteableTypeEnum::Node
    }

    fn id(&self) -> &str {
        self.id
    }
}

#[async_trait::async_trait]
impl Executable for SequenceProbe {
    async fn execute(&self, _ctx: &Ctx, _frame: &Frame) -> LFResult<Value> {
        let index = self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(self
            .outputs
            .get(index)
            .cloned()
            .unwrap_or(Value::Bool(false)))
    }

    fn execute_type(&self) -> ExecuteableTypeEnum {
        ExecuteableTypeEnum::Node
    }

    fn id(&self) -> &str {
        self.id
    }
}

fn sequence_probe(
    id: &'static str,
    outputs: Vec<Value>,
    calls: &Arc<AtomicUsize>,
) -> Arc<dyn Executable> {
    Arc::new(SequenceProbe {
        id,
        outputs,
        calls: Arc::clone(calls),
    })
}

#[tokio::test]
async fn loop_condition_java_state_drives_parallel_body_and_break_execution() {
    let old_calls = Arc::new(AtomicUsize::new(0));
    let body_calls = Arc::new(AtomicUsize::new(0));
    let break_calls = Arc::new(AtomicUsize::new(0));
    let mut condition =
        ForCondition::with_count(4, false, probe("old-body", Value::Null, &old_calls), None);

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
    condition
        .execute_condition(&ctx, &Frame::root())
        .await
        .expect("串行 FOR 应在循环体后响应 BREAK");
    assert_eq!(body_calls.load(Ordering::SeqCst), 2);
    assert_eq!(break_calls.load(Ordering::SeqCst), 2);
}

/// 验证 IteratorCondition 串行迭代、Java 命名入口与非数组错误。
#[tokio::test]
async fn iterator_condition_executes_every_sequential_item_and_rejects_non_array() {
    let iterator_calls = Arc::new(AtomicUsize::new(0));
    let body_calls = Arc::new(AtomicUsize::new(0));
    let condition = IteratorCondition::new(
        probe(
            "iterator",
            serde_json::json!(["a", "b", "c"]),
            &iterator_calls,
        ),
        false,
        probe("body", Value::Null, &body_calls),
        None,
    );
    let ctx = Ctx::new(Arc::new(Slot::new(
        "iterator-sequential".to_string(),
        "loop-chain",
        Value::Null,
    )));

    assert_eq!(
        condition
            .execute_condition(&ctx, &Frame::root())
            .await
            .expect("串行 ITERATOR 应执行完成"),
        Value::Null
    );
    assert_eq!(iterator_calls.load(Ordering::SeqCst), 1);
    assert_eq!(body_calls.load(Ordering::SeqCst), 3);

    let break_calls = Arc::new(AtomicUsize::new(0));
    let break_body_calls = Arc::new(AtomicUsize::new(0));
    let with_break = IteratorCondition::new(
        probe("iterator-break", serde_json::json!([1, 2]), &iterator_calls),
        false,
        probe("break-body", Value::Null, &break_body_calls),
        Some(probe("break", Value::Bool(true), &break_calls)),
    );
    with_break
        .execute(&ctx, &Frame::root())
        .await
        .expect("串行 BREAK 应停止后续迭代");
    assert_eq!(break_body_calls.load(Ordering::SeqCst), 1);
    assert_eq!(break_calls.load(Ordering::SeqCst), 1);

    let invalid = IteratorCondition::new(
        probe("invalid-iterator", Value::Bool(true), &iterator_calls),
        false,
        probe("unused", Value::Null, &body_calls),
        None,
    );
    assert!(matches!(
        invalid.execute(&ctx, &Frame::root()).await,
        Err(LiteflowError::NodeTypeError { expect, .. }) if expect == "array"
    ));

    let denied_calls = Arc::new(AtomicUsize::new(0));
    let denied_body_calls = Arc::new(AtomicUsize::new(0));
    let denied = IteratorCondition::new(
        Arc::new(DeniedProbe {
            id: "denied-iterator",
            calls: Arc::clone(&denied_calls),
        }),
        false,
        probe("denied-body", Value::Null, &denied_body_calls),
        None,
    );
    denied
        .execute(&ctx, &Frame::root())
        .await
        .expect("isAccess=false 应跳过整个 ITERATOR");
    assert_eq!(denied_calls.load(Ordering::SeqCst), 0);
    assert_eq!(denied_body_calls.load(Ordering::SeqCst), 0);
}

/// 验证 IteratorCondition 并行提交后由 BREAK 停止后续迭代。
#[tokio::test]
async fn iterator_condition_parallel_break_waits_for_submitted_body() {
    let iterator_calls = Arc::new(AtomicUsize::new(0));
    let body_calls = Arc::new(AtomicUsize::new(0));
    let break_calls = Arc::new(AtomicUsize::new(0));
    let mut condition = IteratorCondition::new(
        probe("iterator", serde_json::json!([1, 2, 3]), &iterator_calls),
        true,
        probe("body", Value::Null, &body_calls),
        Some(probe("break", Value::Bool(true), &break_calls)),
    );
    condition.set_thread_pool_executor_class(
        "com.yomahub.liteflow.thread.LiteFlowDefaultGlobalExecutorBuilder",
    );
    let ctx = Ctx::new(Arc::new(Slot::new(
        "iterator-parallel".to_string(),
        "loop-chain",
        Value::Null,
    )));

    condition
        .execute(&ctx, &Frame::root())
        .await
        .expect("并行 ITERATOR 应结算已提交任务");
    assert_eq!(iterator_calls.load(Ordering::SeqCst), 1);
    assert_eq!(body_calls.load(Ordering::SeqCst), 1);
    assert_eq!(break_calls.load(Ordering::SeqCst), 1);

    let mut invalid_executor = condition.clone();
    invalid_executor.set_thread_pool_executor_class("missing.IteratorExecutorBuilder");
    assert!(matches!(
        invalid_executor.execute(&ctx, &Frame::root()).await,
        Err(LiteflowError::ThreadExecutorServiceCreate(_))
    ));
}

/// 验证 IteratorCondition 强类型分组与 LoopCondition setter 共用真实执行字段。
#[test]
fn iterator_condition_java_state_and_typed_groups_share_one_source() {
    let calls = Arc::new(AtomicUsize::new(0));
    let mut condition = IteratorCondition::new(
        probe("iterator-a", serde_json::json!([]), &calls),
        false,
        probe("body-a", Value::Null, &calls),
        None,
    );

    assert_eq!(
        condition.get_condition_type(),
        liteflow_core::ConditionTypeEnum::Iterator
    );
    assert_eq!(
        <IteratorCondition as Condition>::condition_type(&condition),
        liteflow_core::ConditionTypeEnum::Iterator
    );
    assert_eq!(condition.get_iterator_node().id(), "iterator-a");
    condition.set_iterator_node(probe("iterator-b", serde_json::json!([]), &calls));
    condition.set_do_executor(probe("body-b", Value::Null, &calls));
    condition.set_break_item(probe("break-b", Value::Bool(false), &calls));
    assert_eq!(condition.get_iterator_node().id(), "iterator-b");
    assert_eq!(condition.get_do_executor().id(), "body-b");
    assert_eq!(
        condition.get_break_item().expect("应存在 BREAK").id(),
        "break-b"
    );
    assert_eq!(condition.get_thread_pool_executor_class(), None);
    assert!(!condition.is_parallel());
    condition.set_parallel(true);
    assert!(condition.is_parallel());
    condition.set_parallel(false);
    assert_eq!(Condition::get_id(&condition), "condition-iterator");
    Condition::set_id(&mut condition, "iterator-condition");
    assert_eq!(Condition::get_id(&condition), "iterator-condition");

    let groups = condition.get_executable_group();
    assert_eq!(groups["ITERATOR_KEY"][0].id(), "iterator-b");
    assert_eq!(groups["DO_KEY"][0].id(), "body-b");
    assert_eq!(groups["BREAK_KEY"][0].id(), "break-b");

    condition.replace_executable_group(
        "ITERATOR_KEY",
        vec![probe("iterator-c", serde_json::json!([]), &calls)],
    );
    condition.replace_executable_group("DO_KEY", vec![probe("body-c", Value::Null, &calls)]);
    condition.replace_executable_group("BREAK_KEY", Vec::new());
    condition.replace_executable_group("CUSTOM_KEY", vec![probe("custom", Value::Null, &calls)]);
    assert_eq!(condition.get_iterator_node().id(), "iterator-c");
    assert_eq!(condition.get_do_executor().id(), "body-c");
    assert!(condition.get_break_item().is_none());
    assert_eq!(
        condition.get_executable_group()["CUSTOM_KEY"][0].id(),
        "custom"
    );
    let mut collected_node_ids = Executable::collect_node_ids(&condition);
    collected_node_ids.sort();
    assert_eq!(collected_node_ids, vec!["body-c", "custom", "iterator-c"]);
    Executable::apply_chain_cmp_data(&condition, "payload");
    assert_eq!(Executable::id(&condition), "ITERATOR");
}

/// 验证 WhileCondition 每轮重新执行条件项，并在串行 BREAK 后停止。
#[tokio::test]
async fn while_condition_rechecks_condition_and_honors_sequential_break() {
    let while_calls = Arc::new(AtomicUsize::new(0));
    let body_calls = Arc::new(AtomicUsize::new(0));
    let condition = WhileCondition::new(
        sequence_probe(
            "while",
            vec![Value::Bool(true), Value::Bool(true), Value::Bool(false)],
            &while_calls,
        ),
        false,
        probe("body", Value::Null, &body_calls),
        None,
    );
    let ctx = Ctx::new(Arc::new(Slot::new(
        "while-sequential".to_string(),
        "loop-chain",
        Value::Null,
    )));

    assert_eq!(
        condition
            .execute_condition(&ctx, &Frame::root())
            .await
            .expect("串行 WHILE 应在条件返回 false 后结束"),
        Value::Null
    );
    assert_eq!(while_calls.load(Ordering::SeqCst), 3);
    assert_eq!(body_calls.load(Ordering::SeqCst), 2);

    let break_while_calls = Arc::new(AtomicUsize::new(0));
    let break_body_calls = Arc::new(AtomicUsize::new(0));
    let break_calls = Arc::new(AtomicUsize::new(0));
    let with_break = WhileCondition::new(
        sequence_probe(
            "while-break",
            vec![Value::Bool(true), Value::Bool(true)],
            &break_while_calls,
        ),
        false,
        probe("break-body", Value::Null, &break_body_calls),
        Some(probe("break", Value::Bool(true), &break_calls)),
    );
    with_break
        .execute(&ctx, &Frame::root())
        .await
        .expect("串行 WHILE 应在循环体执行后响应 BREAK");
    assert_eq!(break_while_calls.load(Ordering::SeqCst), 1);
    assert_eq!(break_body_calls.load(Ordering::SeqCst), 1);
    assert_eq!(break_calls.load(Ordering::SeqCst), 1);

    let denied_calls = Arc::new(AtomicUsize::new(0));
    let denied_body_calls = Arc::new(AtomicUsize::new(0));
    let denied = WhileCondition::new(
        Arc::new(DeniedProbe {
            id: "denied-while",
            calls: Arc::clone(&denied_calls),
        }),
        false,
        probe("denied-body", Value::Null, &denied_body_calls),
        None,
    );
    denied
        .execute(&ctx, &Frame::root())
        .await
        .expect("isAccess=false 应跳过整个 WHILE");
    assert_eq!(denied_calls.load(Ordering::SeqCst), 0);
    assert_eq!(denied_body_calls.load(Ordering::SeqCst), 0);
}

/// 验证 WhileCondition 并行提交、线程池失败与条件结果类型约束。
#[tokio::test]
async fn while_condition_parallel_break_and_errors_match_java_semantics() {
    let while_calls = Arc::new(AtomicUsize::new(0));
    let body_calls = Arc::new(AtomicUsize::new(0));
    let break_calls = Arc::new(AtomicUsize::new(0));
    let mut condition = WhileCondition::new(
        sequence_probe(
            "while",
            vec![Value::Bool(true), Value::Bool(true)],
            &while_calls,
        ),
        true,
        probe("body", Value::Null, &body_calls),
        Some(probe("break", Value::Bool(true), &break_calls)),
    );
    condition.set_thread_pool_executor_class(
        "com.yomahub.liteflow.thread.LiteFlowDefaultGlobalExecutorBuilder",
    );
    let ctx = Ctx::new(Arc::new(Slot::new(
        "while-parallel".to_string(),
        "loop-chain",
        Value::Null,
    )));

    condition
        .execute(&ctx, &Frame::root())
        .await
        .expect("并行 WHILE 应结算 BREAK 前已提交的循环体");
    assert_eq!(while_calls.load(Ordering::SeqCst), 1);
    assert_eq!(body_calls.load(Ordering::SeqCst), 1);
    assert_eq!(break_calls.load(Ordering::SeqCst), 1);

    let natural_while_calls = Arc::new(AtomicUsize::new(0));
    let natural_body_calls = Arc::new(AtomicUsize::new(0));
    let natural_stop = WhileCondition::new(
        sequence_probe(
            "while-natural-stop",
            vec![Value::Bool(true), Value::Bool(false)],
            &natural_while_calls,
        ),
        true,
        probe("natural-body", Value::Null, &natural_body_calls),
        None,
    );
    natural_stop
        .execute(&ctx, &Frame::root())
        .await
        .expect("并行 WHILE 应在下一轮条件为 false 时结算已提交任务");
    assert_eq!(natural_while_calls.load(Ordering::SeqCst), 2);
    assert_eq!(natural_body_calls.load(Ordering::SeqCst), 1);

    let mut invalid_executor = condition.clone();
    invalid_executor.set_thread_pool_executor_class("missing.WhileExecutorBuilder");
    assert!(matches!(
        invalid_executor.execute(&ctx, &Frame::root()).await,
        Err(LiteflowError::ThreadExecutorServiceCreate(_))
    ));

    let invalid_calls = Arc::new(AtomicUsize::new(0));
    let invalid = WhileCondition::new(
        probe(
            "invalid-while",
            Value::String("true".to_string()),
            &invalid_calls,
        ),
        false,
        probe("unused", Value::Null, &body_calls),
        None,
    );
    assert!(matches!(
        invalid.execute(&ctx, &Frame::root()).await,
        Err(LiteflowError::NodeTypeError { expect, .. }) if expect == "boolean"
    ));
}

/// 验证 WhileCondition Java 命名状态、强类型分组和通用 Condition API。
#[test]
fn while_condition_java_state_and_typed_groups_share_one_source() {
    let calls = Arc::new(AtomicUsize::new(0));
    let mut condition = WhileCondition::new(
        probe("while-a", Value::Bool(false), &calls),
        false,
        probe("body-a", Value::Null, &calls),
        None,
    );

    assert_eq!(
        condition.get_condition_type(),
        liteflow_core::ConditionTypeEnum::While
    );
    assert_eq!(
        condition.condition_type(),
        liteflow_core::ConditionTypeEnum::While
    );
    assert_eq!(
        <WhileCondition as Condition>::condition_type(&condition),
        liteflow_core::ConditionTypeEnum::While
    );
    assert_eq!(condition.get_while_item().id(), "while-a");
    condition.set_while_item(probe("while-b", Value::Bool(false), &calls));
    condition.set_do_executor(probe("body-b", Value::Null, &calls));
    condition.set_break_item(probe("break-b", Value::Bool(false), &calls));
    condition.set_thread_pool_executor_class("while.ExecutorBuilder");
    condition.set_parallel(true);
    assert_eq!(condition.get_while_item().id(), "while-b");
    assert_eq!(condition.get_do_executor().id(), "body-b");
    assert_eq!(
        condition.get_break_item().expect("应存在 BREAK").id(),
        "break-b"
    );
    assert_eq!(
        condition.get_thread_pool_executor_class(),
        Some("while.ExecutorBuilder")
    );
    assert!(condition.is_parallel());
    condition.set_parallel(false);
    assert!(!condition.is_parallel());
    Condition::set_id(&mut condition, "while-condition");
    assert_eq!(Condition::get_id(&condition), "while-condition");

    let groups = condition.get_executable_group();
    assert_eq!(groups["WHILE_KEY"][0].id(), "while-b");
    assert_eq!(groups["DO_KEY"][0].id(), "body-b");
    assert_eq!(groups["BREAK_KEY"][0].id(), "break-b");

    condition.replace_executable_group(
        "WHILE_KEY",
        vec![probe("while-c", Value::Bool(false), &calls)],
    );
    condition.replace_executable_group("DO_KEY", vec![probe("body-c", Value::Null, &calls)]);
    condition.replace_executable_group("BREAK_KEY", Vec::new());
    condition.replace_executable_group("CUSTOM_KEY", vec![probe("custom", Value::Null, &calls)]);
    assert_eq!(condition.get_while_item().id(), "while-c");
    assert_eq!(condition.get_do_executor().id(), "body-c");
    assert!(condition.get_break_item().is_none());
    assert_eq!(
        condition.get_executable_group()["CUSTOM_KEY"][0].id(),
        "custom"
    );
    let mut collected_node_ids = Executable::collect_node_ids(&condition);
    collected_node_ids.sort();
    assert_eq!(collected_node_ids, vec!["body-c", "custom", "while-c"]);
    Executable::apply_chain_cmp_data(&condition, "payload");
    assert_eq!(Executable::id(&condition), "WHILE");
}

/// 验证动态 FOR 节点只接受 Java Integer 语义，并保留 isAccess 短路。
#[tokio::test]
async fn for_condition_dynamic_count_requires_integer_and_honors_access() {
    let count_calls = Arc::new(AtomicUsize::new(0));
    let body_calls = Arc::new(AtomicUsize::new(0));
    let condition = ForCondition::new(
        probe("for-count", Value::Number(3.into()), &count_calls),
        false,
        probe("body", Value::Null, &body_calls),
        None,
    );
    let ctx = Ctx::new(Arc::new(Slot::new(
        "for-dynamic".to_string(),
        "loop-chain",
        Value::Null,
    )));

    condition
        .execute_condition(&ctx, &Frame::root())
        .await
        .expect("动态整数 FOR 应执行指定次数");
    assert_eq!(count_calls.load(Ordering::SeqCst), 1);
    assert_eq!(body_calls.load(Ordering::SeqCst), 3);

    let denied_calls = Arc::new(AtomicUsize::new(0));
    let denied_body_calls = Arc::new(AtomicUsize::new(0));
    let denied = ForCondition::new(
        Arc::new(DeniedProbe {
            id: "denied",
            calls: Arc::clone(&denied_calls),
        }),
        false,
        probe("denied-body", Value::Null, &denied_body_calls),
        None,
    );
    assert_eq!(
        denied
            .execute(&ctx, &Frame::root())
            .await
            .expect("isAccess=false 应跳过整个 FOR"),
        Value::Null
    );
    assert_eq!(denied_calls.load(Ordering::SeqCst), 0);
    assert_eq!(denied_body_calls.load(Ordering::SeqCst), 0);

    for invalid_value in [
        Value::String("3".to_string()),
        serde_json::json!(1.5),
        Value::Bool(true),
    ] {
        let invalid = ForCondition::new(
            probe("invalid-count", invalid_value, &count_calls),
            false,
            probe("unused", Value::Null, &body_calls),
            None,
        );
        assert!(matches!(
            invalid.execute(&ctx, &Frame::root()).await,
            Err(LiteflowError::NodeTypeError { expect, .. }) if expect == "integer"
        ));
    }

    let negative_body_calls = Arc::new(AtomicUsize::new(0));
    let negative = ForCondition::new(
        probe("negative-count", serde_json::json!(-2), &count_calls),
        false,
        probe("negative-body", Value::Null, &negative_body_calls),
        None,
    );
    negative
        .execute(&ctx, &Frame::root())
        .await
        .expect("Java 负整数 FOR 的循环条件初始即为 false");
    assert_eq!(negative_body_calls.load(Ordering::SeqCst), 0);
}

/// 验证 ForCondition Java 状态、强类型分组与并行线程池错误。
#[tokio::test]
async fn for_condition_java_state_groups_and_parallel_errors_share_one_source() {
    let calls = Arc::new(AtomicUsize::new(0));
    let mut condition = ForCondition::new(
        probe("for-a", Value::Number(1.into()), &calls),
        false,
        probe("body-a", Value::Null, &calls),
        None,
    );

    assert_eq!(
        condition.get_condition_type(),
        liteflow_core::ConditionTypeEnum::For
    );
    assert_eq!(
        condition.condition_type(),
        liteflow_core::ConditionTypeEnum::For
    );
    assert_eq!(
        <ForCondition as Condition>::condition_type(&condition),
        liteflow_core::ConditionTypeEnum::For
    );
    assert_eq!(
        condition.get_for_node().expect("应存在动态 FOR 节点").id(),
        "for-a"
    );
    condition.set_for_node(probe("for-b", Value::Number(1.into()), &calls));
    condition.set_do_executor(probe("body-b", Value::Null, &calls));
    condition.set_break_item(probe("break-b", Value::Bool(false), &calls));
    condition.set_thread_pool_executor_class("missing.ForExecutorBuilder");
    condition.set_parallel(true);
    assert_eq!(
        condition.get_thread_pool_executor_class(),
        Some("missing.ForExecutorBuilder")
    );
    assert!(matches!(
        condition
            .execute(
                &Ctx::new(Arc::new(Slot::new(
                    "for-parallel-error".to_string(),
                    "loop-chain",
                    Value::Null,
                ))),
                &Frame::root(),
            )
            .await,
        Err(LiteflowError::ThreadExecutorServiceCreate(_))
    ));
    condition.set_parallel(false);

    let groups = condition.get_executable_group();
    assert_eq!(groups["FOR_KEY"][0].id(), "for-b");
    assert_eq!(groups["DO_KEY"][0].id(), "body-b");
    assert_eq!(groups["BREAK_KEY"][0].id(), "break-b");
    condition.replace_executable_group(
        "FOR_KEY",
        vec![probe("for-c", Value::Number(0.into()), &calls)],
    );
    condition.replace_executable_group("DO_KEY", vec![probe("body-c", Value::Null, &calls)]);
    condition.replace_executable_group("BREAK_KEY", Vec::new());
    condition.replace_executable_group("CUSTOM_KEY", vec![probe("custom", Value::Null, &calls)]);
    assert_eq!(
        condition.get_for_node().expect("替换后仍有 FOR 节点").id(),
        "for-c"
    );
    assert_eq!(condition.get_do_executor().id(), "body-c");
    assert!(condition.get_break_item().is_none());
    Condition::set_id(&mut condition, "for-condition");
    assert_eq!(Condition::get_id(&condition), "for-condition");
    let mut node_ids = Executable::collect_node_ids(&condition);
    node_ids.sort();
    assert_eq!(node_ids, vec!["body-c", "custom", "for-c"]);
    Executable::apply_chain_cmp_data(&condition, "payload");
    assert_eq!(Executable::id(&condition), "FOR");

    condition.replace_executable_group("FOR_KEY", Vec::new());
    assert!(condition.get_for_node().is_none());
}

/// 验证 SWITCH 的 ID、ID+tag、纯 tag 与 `tag:` 前缀路由规则。
#[tokio::test]
async fn switch_condition_routes_by_java_id_and_tag_rules() {
    let calls = Arc::new(AtomicUsize::new(0));
    let ctx = Ctx::new(Arc::new(Slot::new(
        "switch-routing".to_string(),
        "switch-chain",
        Value::Null,
    )));

    for (selector, expected) in [
        ("alpha", "alpha-result"),
        ("beta:blue", "beta-blue-result"),
        (":blue", "beta-blue-result"),
        ("tag:blue", "beta-blue-result"),
    ] {
        let condition = SwitchCondition::new(
            probe("switch", Value::String(selector.to_string()), &calls),
            vec![
                tagged_probe("alpha", "red", Value::String("alpha-result".into()), &calls),
                tagged_probe(
                    "beta",
                    "blue",
                    Value::String("beta-blue-result".into()),
                    &calls,
                ),
            ],
            None,
        );
        assert_eq!(
            condition
                .execute_condition(&ctx, &Frame::root())
                .await
                .expect("SWITCH 应按 Java ID/tag 规则命中目标"),
            Value::String(expected.to_string())
        );
    }
}

/// 验证 SWITCH 默认分支、空白语义、无目标、类型错误和 PRE 目标禁令。
#[tokio::test]
async fn switch_condition_default_and_error_paths_match_java() {
    let calls = Arc::new(AtomicUsize::new(0));
    let ctx = Ctx::new(Arc::new(Slot::new(
        "switch-errors".to_string(),
        "switch-chain",
        Value::Null,
    )));

    for selector in [
        Value::String("missing".to_string()),
        Value::String("   ".to_string()),
        Value::Null,
    ] {
        let condition = SwitchCondition::new(
            probe("switch", selector, &calls),
            vec![probe("   ", Value::String("wrong".into()), &calls)],
            Some(probe(
                "default",
                Value::String("default-result".into()),
                &calls,
            )),
        );
        assert_eq!(
            condition
                .execute(&ctx, &Frame::root())
                .await
                .expect("未命中或空白 SWITCH 值应执行 DEFAULT"),
            Value::String("default-result".to_string())
        );
    }

    let no_target = SwitchCondition::new(
        probe("switch", Value::String("missing".into()), &calls),
        Vec::new(),
        None,
    );
    assert!(matches!(
        no_target.execute(&ctx, &Frame::root()).await,
        Err(LiteflowError::NoSwitchTarget(target)) if target == "missing"
    ));

    let invalid_type =
        SwitchCondition::new(probe("switch", Value::Bool(true), &calls), Vec::new(), None);
    assert!(matches!(
        invalid_type.execute(&ctx, &Frame::root()).await,
        Err(LiteflowError::NodeTypeError { expect, .. }) if expect == "string"
    ));

    let denied_calls = Arc::new(AtomicUsize::new(0));
    let denied_target_calls = Arc::new(AtomicUsize::new(0));
    let denied = SwitchCondition::new(
        Arc::new(DeniedProbe {
            id: "denied-switch",
            calls: Arc::clone(&denied_calls),
        }),
        vec![probe("target", Value::Null, &denied_target_calls)],
        None,
    );
    denied
        .execute(&ctx, &Frame::root())
        .await
        .expect("isAccess=false 应跳过整个 SWITCH");
    assert_eq!(denied_calls.load(Ordering::SeqCst), 0);
    assert_eq!(denied_target_calls.load(Ordering::SeqCst), 0);

    let pre_target: Arc<dyn Executable> =
        Arc::new(PreCondition::new(probe("inside-pre", Value::Null, &calls)));
    let invalid_target = SwitchCondition::new(
        probe("switch", Value::String("PRE".into()), &calls),
        vec![pre_target],
        None,
    );
    assert!(matches!(
        invalid_target.execute(&ctx, &Frame::root()).await,
        Err(LiteflowError::TargetCannotBePreOrFinally(_))
    ));
}

/// 验证 SwitchCondition Java 命名 API 与强类型可执行对象分组。
#[test]
fn switch_condition_java_state_and_typed_groups_share_one_source() {
    let calls = Arc::new(AtomicUsize::new(0));
    let mut condition = SwitchCondition::new(
        probe("switch-a", Value::String("target-a".into()), &calls),
        vec![probe("target-a", Value::Null, &calls)],
        None,
    );

    assert_eq!(
        condition.get_condition_type(),
        liteflow_core::ConditionTypeEnum::Switch
    );
    assert_eq!(
        condition.condition_type(),
        liteflow_core::ConditionTypeEnum::Switch
    );
    assert_eq!(
        <SwitchCondition as Condition>::condition_type(&condition),
        liteflow_core::ConditionTypeEnum::Switch
    );
    condition.set_switch_node(probe("switch-b", Value::String("target-b".into()), &calls));
    condition.add_target_item(probe("target-b", Value::Null, &calls));
    condition.set_default_executor(probe("default-b", Value::Null, &calls));
    assert_eq!(condition.get_switch_node().id(), "switch-b");
    assert_eq!(condition.get_target_list().len(), 2);
    assert_eq!(
        condition
            .get_default_executor()
            .expect("应存在 DEFAULT")
            .id(),
        "default-b"
    );
    Condition::set_id(&mut condition, "switch-condition");
    assert_eq!(Condition::get_id(&condition), "switch-condition");

    let groups = condition.get_executable_group();
    assert_eq!(groups["SWITCH_KEY"][0].id(), "switch-b");
    assert_eq!(groups["SWITCH_TARGET_KEY"].len(), 2);
    assert_eq!(groups["SWITCH_DEFAULT_KEY"][0].id(), "default-b");
    condition.replace_executable_group("SWITCH_KEY", vec![probe("switch-c", Value::Null, &calls)]);
    condition.replace_executable_group(
        "SWITCH_TARGET_KEY",
        vec![probe("target-c", Value::Null, &calls)],
    );
    condition.replace_executable_group("SWITCH_DEFAULT_KEY", Vec::new());
    condition.replace_executable_group("CUSTOM_KEY", vec![probe("custom", Value::Null, &calls)]);
    assert_eq!(condition.get_switch_node().id(), "switch-c");
    assert_eq!(condition.get_target_list()[0].id(), "target-c");
    assert!(condition.get_default_executor().is_none());
    assert_eq!(
        condition.get_executable_group()["CUSTOM_KEY"][0].id(),
        "custom"
    );
    let mut node_ids = Executable::collect_node_ids(&condition);
    node_ids.sort();
    assert_eq!(node_ids, vec!["custom", "switch-c", "target-c"]);
    Executable::apply_chain_cmp_data(&condition, "payload");
    assert_eq!(Executable::id(&condition), "SWITCH");
}
