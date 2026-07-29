use liteflow_core::exception::{LFResult, LiteflowError};
use liteflow_core::flow::element::chain::Chain;
use liteflow_core::flow::element::condition::abstract_condition::AbstractCondition;
use liteflow_core::flow::element::condition::and_or_condition::AndOrCondition;
use liteflow_core::flow::element::condition::catch_condition::CatchCondition;
use liteflow_core::flow::element::condition::finally_condition::FinallyCondition;
use liteflow_core::flow::element::condition::for_condition::ForCondition;
use liteflow_core::flow::element::condition::if_condition::IfCondition;
use liteflow_core::flow::element::condition::iterator_condition::IteratorCondition;
use liteflow_core::flow::element::condition::not_condition::NotCondition;
use liteflow_core::flow::element::condition::pre_condition::PreCondition;
use liteflow_core::flow::element::condition::switch_condition::SwitchCondition;
use liteflow_core::flow::element::condition::then_condition::ThenCondition;
use liteflow_core::flow::element::condition::timeout_condition::TimeoutCondition;
use liteflow_core::flow::element::condition::when_condition::WhenCondition;
use liteflow_core::flow::element::condition::while_condition::WhileCondition;
use liteflow_core::flow::element::condition::{BooleanConditionTypeEnum, Condition};
use liteflow_core::flow::element::executable::Executable;
use liteflow_core::slot::{Ctx, Frame, Slot};
use liteflow_core::{ConditionTypeEnum, ParallelStrategyEnum, TimeUnit};
use serde_json::{Value, json};
use std::collections::{HashSet, VecDeque};
use std::sync::{Arc, Mutex};

struct Probe {
    id: &'static str,
    tag: Option<&'static str>,
    output: Value,
    fail: bool,
    calls: Arc<Mutex<Vec<&'static str>>>,
}

struct SequenceProbe {
    id: &'static str,
    outputs: Mutex<VecDeque<Value>>,
    calls: Arc<Mutex<Vec<&'static str>>>,
}

#[derive(Clone, Copy)]
enum StackProbeOutcome {
    Success,
    Failure,
    ChainEnd,
}

struct StackProbe {
    id: &'static str,
    outcome: StackProbeOutcome,
    observed_stack: Arc<Mutex<Vec<Vec<String>>>>,
}

struct SlowProbe;

#[async_trait::async_trait]
impl Executable for SlowProbe {
    async fn execute(&self, _ctx: &Ctx, _frame: &Frame) -> LFResult<Value> {
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        Ok(json!("late"))
    }
}

#[async_trait::async_trait]
impl Executable for StackProbe {
    async fn execute(&self, ctx: &Ctx, frame: &Frame) -> LFResult<Value> {
        let stack = ctx
            .inner
            .get_condition_stack(frame)
            .iter()
            .map(|condition| Condition::get_id(condition.as_ref()))
            .collect();
        self.observed_stack
            .lock()
            .expect("Condition 栈观察锁不应中毒")
            .push(stack);
        match self.outcome {
            StackProbeOutcome::Success => Ok(Value::Null),
            StackProbeOutcome::Failure => Err(LiteflowError::Custom(format!("{} failed", self.id))),
            StackProbeOutcome::ChainEnd => Err(LiteflowError::ChainEnd("chain end".to_string())),
        }
    }

    fn id(&self) -> &str {
        self.id
    }

    fn execute_type(&self) -> liteflow_core::ExecuteableTypeEnum {
        liteflow_core::ExecuteableTypeEnum::Node
    }
}

impl SequenceProbe {
    fn new(
        id: &'static str,
        outputs: impl IntoIterator<Item = Value>,
        calls: &Arc<Mutex<Vec<&'static str>>>,
    ) -> Arc<dyn Executable> {
        Arc::new(Self {
            id,
            outputs: Mutex::new(outputs.into_iter().collect()),
            calls: Arc::clone(calls),
        })
    }
}

#[async_trait::async_trait]
impl Executable for SequenceProbe {
    async fn execute(&self, _ctx: &Ctx, _frame: &Frame) -> LFResult<Value> {
        self.calls.lock().expect("调用记录锁不应中毒").push(self.id);
        Ok(self
            .outputs
            .lock()
            .expect("序列结果锁不应中毒")
            .pop_front()
            .unwrap_or(Value::Bool(false)))
    }

    fn id(&self) -> &str {
        self.id
    }

    fn execute_type(&self) -> liteflow_core::ExecuteableTypeEnum {
        liteflow_core::ExecuteableTypeEnum::Node
    }
}

impl Probe {
    fn success(
        id: &'static str,
        output: Value,
        calls: &Arc<Mutex<Vec<&'static str>>>,
    ) -> Arc<dyn Executable> {
        Arc::new(Self {
            id,
            tag: None,
            output,
            fail: false,
            calls: Arc::clone(calls),
        })
    }

    fn tagged(
        id: &'static str,
        tag: &'static str,
        output: Value,
        calls: &Arc<Mutex<Vec<&'static str>>>,
    ) -> Arc<dyn Executable> {
        Arc::new(Self {
            id,
            tag: Some(tag),
            output,
            fail: false,
            calls: Arc::clone(calls),
        })
    }

    fn failure(id: &'static str, calls: &Arc<Mutex<Vec<&'static str>>>) -> Arc<dyn Executable> {
        Arc::new(Self {
            id,
            tag: None,
            output: Value::Null,
            fail: true,
            calls: Arc::clone(calls),
        })
    }
}

#[async_trait::async_trait]
impl Executable for Probe {
    async fn execute(&self, _ctx: &Ctx, _frame: &Frame) -> LFResult<Value> {
        self.calls.lock().expect("调用记录锁不应中毒").push(self.id);
        if self.fail {
            Err(LiteflowError::Custom(format!("{} failed", self.id)))
        } else {
            Ok(self.output.clone())
        }
    }

    fn id(&self) -> &str {
        self.id
    }

    fn tag(&self) -> Option<&str> {
        self.tag
    }

    fn execute_type(&self) -> liteflow_core::ExecuteableTypeEnum {
        liteflow_core::ExecuteableTypeEnum::Node
    }
}

fn execution_context() -> (Ctx, Frame) {
    let slot = Arc::new(Slot::new(
        "condition-api".to_string(),
        "condition-chain",
        Value::Null,
    ));
    (Ctx::new(slot), Frame::root())
}

#[tokio::test]
async fn condition_base_java_metadata_is_owned_cloned_and_used_by_real_objects() {
    let calls = Arc::new(Mutex::new(Vec::new()));
    let (ctx, frame) = execution_context();
    let mut condition = IfCondition::new(
        Probe::success("condition-flag", Value::Bool(true), &calls),
        Probe::success("condition-body", json!("executed"), &calls),
        Vec::new(),
        None,
    );

    // Java Condition 抽象类的元数据必须由对象自身持有，不能落入全局兼容表。
    assert_eq!(
        Condition::get_id(&condition),
        format!("condition-{}", ConditionTypeEnum::If.get_name())
    );
    assert_eq!(
        Condition::get_execute_type(&condition),
        liteflow_core::ExecuteableTypeEnum::Condition
    );
    assert_eq!(
        Condition::get_condition_type(&condition),
        ConditionTypeEnum::If
    );
    Condition::set_id(&mut condition, "checkout-condition");
    Condition::set_tag(&mut condition, "blue");
    Condition::set_curr_chain_id(&mut condition, "checkout-chain");
    Condition::put_bind_data(&mut condition, "tenant", "north");
    Condition::put_bind_data(&mut condition, "tenant", "south");

    assert_eq!(Condition::get_id(&condition), "checkout-condition");
    assert_eq!(Condition::get_tag(&condition), Some("blue"));
    assert_eq!(
        Condition::get_curr_chain_name(&condition),
        Some("checkout-chain")
    );
    assert_eq!(
        Condition::get_curr_chain_id(&condition),
        Some("checkout-chain")
    );
    assert_eq!(
        Condition::get_bind_data(&condition, "tenant"),
        Some("south")
    );
    assert!(Condition::has_bind_data(&condition, "tenant"));
    assert!(!Condition::has_bind_data(&condition, "missing"));

    let cloned = condition.clone();
    Condition::set_id(&mut condition, "changed-after-clone");
    Condition::put_bind_data(&mut condition, "tenant", "east");
    assert_eq!(Condition::get_id(&cloned), "checkout-condition");
    assert_eq!(Condition::get_bind_data(&cloned, "tenant"), Some("south"));

    assert_eq!(
        Condition::execute_condition(&cloned, &ctx, &frame)
            .await
            .expect("公共 executeCondition 应委托真实 IF 执行"),
        json!("executed")
    );
    assert_eq!(
        Condition::get_executable_one(&cloned, "IF_KEY")
            .expect("IF_KEY 应映射到真实 if_item")
            .id(),
        "condition-flag"
    );
    assert_eq!(
        Condition::get_executable_list(&cloned, "IF_TRUE_CASE_KEY")[0].id(),
        "condition-body"
    );
    let mut condition_node_ids = Condition::get_all_node_in_condition(&cloned);
    condition_node_ids.sort();
    assert_eq!(
        condition_node_ids,
        vec!["condition-body".to_string(), "condition-flag".to_string()]
    );

    let mut sequence = ThenCondition::new();
    Condition::set_executable_list(
        &mut sequence,
        vec![Probe::success("base-main-a", Value::Null, &calls)],
    );
    Condition::add_executable(
        &mut sequence,
        Probe::success("base-main-b", Value::Null, &calls),
    );
    Condition::add_executable_to_group(
        &mut sequence,
        "CUSTOM_KEY",
        Probe::success("custom-only", Value::Null, &calls),
    );
    assert_eq!(
        Condition::get_executable_list(&sequence, "DEFAULT_KEY")
            .iter()
            .map(|item| item.id())
            .collect::<Vec<_>>(),
        vec!["base-main-a", "base-main-b"]
    );
    assert_eq!(
        Condition::get_executable_group(&sequence)["CUSTOM_KEY"][0].id(),
        "custom-only"
    );
    Condition::execute_condition(&sequence, &ctx, &frame)
        .await
        .expect("公共分组 setter 添加的主体必须进入真实 THEN 执行");
    let execution_calls = calls.lock().expect("调用记录锁不应中毒");
    assert!(execution_calls.contains(&"base-main-a"));
    assert!(execution_calls.contains(&"base-main-b"));
    assert!(!execution_calls.contains(&"custom-only"));
    drop(execution_calls);

    let mut abstract_condition = AbstractCondition::new("abstract-a");
    assert_eq!(
        Condition::get_curr_chain_id(&abstract_condition),
        Some("abstract-a")
    );
    abstract_condition.set_curr_chain_id("abstract-b");
    assert_eq!(
        Condition::get_curr_chain_id(&abstract_condition),
        Some("abstract-b")
    );
    let error = Condition::execute_condition(&abstract_condition, &ctx, &frame)
        .await
        .expect_err("抽象条件仍应拒绝真实执行");
    assert!(error.to_string().contains("abstract-b"));
}

#[tokio::test]
async fn condition_execute_lifecycle_pushes_records_errors_and_always_pops() {
    let observed_stack = Arc::new(Mutex::new(Vec::new()));
    let (ctx, frame) = execution_context();
    let mut success = ThenCondition::new();
    Condition::set_id(&mut success, "outer-success");
    success.add_executable(Arc::new(StackProbe {
        id: "success-node",
        outcome: StackProbeOutcome::Success,
        observed_stack: Arc::clone(&observed_stack),
    }));

    success
        .execute(&ctx, &frame)
        .await
        .expect("成功 Condition 应正常返回");
    assert_eq!(
        observed_stack
            .lock()
            .expect("Condition 栈观察锁不应中毒")
            .as_slice(),
        &[vec!["outer-success".to_string()]]
    );
    assert!(ctx.inner.get_condition_stack(&frame).is_empty());

    let (failure_ctx, failure_frame) = execution_context();
    let mut failure = ThenCondition::new();
    Condition::set_id(&mut failure, "outer-failure");
    failure.add_executable(Arc::new(StackProbe {
        id: "failure-node",
        outcome: StackProbeOutcome::Failure,
        observed_stack: Arc::clone(&observed_stack),
    }));
    assert!(failure.execute(&failure_ctx, &failure_frame).await.is_err());
    assert_eq!(
        failure_ctx.inner.get_exception().as_deref(),
        Some("failure-node failed")
    );
    assert!(
        failure_ctx
            .inner
            .get_condition_stack(&failure_frame)
            .is_empty()
    );

    let (chain_end_ctx, chain_end_frame) = execution_context();
    let mut chain_end = ThenCondition::new();
    Condition::set_id(&mut chain_end, "outer-chain-end");
    chain_end.add_executable(Arc::new(StackProbe {
        id: "chain-end-node",
        outcome: StackProbeOutcome::ChainEnd,
        observed_stack,
    }));
    assert!(matches!(
        chain_end.execute(&chain_end_ctx, &chain_end_frame).await,
        Err(LiteflowError::ChainEnd(_))
    ));
    assert_eq!(chain_end_ctx.inner.get_exception(), None);
    assert!(
        chain_end_ctx
            .inner
            .get_condition_stack(&chain_end_frame)
            .is_empty()
    );
}

#[tokio::test]
async fn timeout_condition_java_entry_enforces_real_deadline() {
    let (ctx, frame) = execution_context();
    let condition = TimeoutCondition::new(Arc::new(SlowProbe), 1);
    let error = condition
        .execute_condition(&ctx, &frame)
        .await
        .expect_err("超过最大等待时间必须返回 WHEN 超时错误");
    assert!(matches!(error, LiteflowError::WhenTimeout(_)));
}

#[test]
fn get_all_node_in_condition_recurses_into_condition_and_chain() {
    let calls = Arc::new(Mutex::new(Vec::new()));
    let mut nested_condition = ThenCondition::new();
    nested_condition.add_executable(Probe::success("nested-a", Value::Null, &calls));
    nested_condition.add_executable(Probe::success("nested-b", Value::Null, &calls));
    nested_condition.add_executable(Probe::success("nested-a", Value::Null, &calls));

    let nested_chain: Arc<dyn Executable> =
        Arc::new(Chain::new("nested-chain", vec![Arc::new(nested_condition)]));
    let mut outer = ThenCondition::new();
    outer.add_executable(Probe::success("outer-node", Value::Null, &calls));
    outer.add_executable(nested_chain);

    assert_eq!(
        Condition::get_all_node_in_condition(&outer),
        vec![
            "outer-node".to_string(),
            "nested-a".to_string(),
            "nested-b".to_string(),
            "nested-a".to_string(),
        ]
    );
}

#[tokio::test]
async fn if_switch_and_catch_java_named_methods_drive_real_execution() {
    let calls = Arc::new(Mutex::new(Vec::new()));
    let (ctx, frame) = execution_context();

    let mut if_condition = IfCondition::new(
        Probe::success("if-false", Value::Bool(false), &calls),
        Probe::success("old-true", json!("old"), &calls),
        Vec::new(),
        None,
    );
    if_condition.set_if_item(Probe::success("if-true", Value::Bool(true), &calls));
    if_condition.set_true_case_executable_item(Probe::success(
        "new-true",
        json!("true-result"),
        &calls,
    ));
    if_condition.set_false_case_executable_item(Probe::success(
        "false",
        json!("false-result"),
        &calls,
    ));
    assert_eq!(if_condition.get_condition_type(), ConditionTypeEnum::If);
    assert_eq!(if_condition.get_if_item().id(), "if-true");
    assert_eq!(
        if_condition
            .get_true_case_executable_item()
            .expect("true 分支应已设置")
            .id(),
        "new-true"
    );
    assert_eq!(
        if_condition
            .get_false_case_executable_item()
            .expect("false 分支应已设置")
            .id(),
        "false"
    );
    assert_eq!(
        if_condition
            .execute_condition(&ctx, &frame)
            .await
            .expect("IF 应成功执行"),
        json!("true-result")
    );

    let mut switch_condition = SwitchCondition::new(
        Probe::success("old-switch", json!("missing"), &calls),
        vec![Probe::success("target-a", json!("a"), &calls)],
        None,
    );
    switch_condition.set_switch_node(Probe::success("switch", json!("target-b:blue"), &calls));
    switch_condition.add_target_item(Probe::tagged("target-b", "blue", json!("selected"), &calls));
    switch_condition.set_default_executor(Probe::success(
        "default",
        json!("default-result"),
        &calls,
    ));
    assert_eq!(
        switch_condition.get_condition_type(),
        ConditionTypeEnum::Switch
    );
    assert_eq!(switch_condition.get_switch_node().id(), "switch");
    assert_eq!(switch_condition.get_target_list().len(), 2);
    assert_eq!(
        switch_condition
            .get_default_executor()
            .expect("默认目标应已设置")
            .id(),
        "default"
    );
    assert_eq!(
        switch_condition
            .execute_condition(&ctx, &frame)
            .await
            .expect("SWITCH 应命中 tag 目标"),
        json!("selected")
    );

    let mut catch_condition =
        CatchCondition::new(Probe::success("old-catch", Value::Null, &calls), None);
    catch_condition.set_catch_item(Probe::failure("failure", &calls));
    catch_condition.set_do_item(Probe::success("recover", json!("recovered"), &calls));
    assert_eq!(
        catch_condition.get_condition_type(),
        ConditionTypeEnum::Catch
    );
    assert_eq!(catch_condition.get_catch_item().id(), "failure");
    assert_eq!(
        catch_condition
            .get_do_item()
            .expect("DO 恢复项应已设置")
            .id(),
        "recover"
    );
    assert_eq!(
        catch_condition
            .execute_condition(&ctx, &frame)
            .await
            .expect("CATCH 应执行恢复项"),
        json!("recovered")
    );
    assert!(ctx.inner.get_exception().is_none());
}

#[tokio::test]
async fn not_loop_and_sequence_java_named_methods_share_runtime_state() {
    let calls = Arc::new(Mutex::new(Vec::new()));
    let (ctx, frame) = execution_context();

    let mut not_condition =
        NotCondition::new(Probe::success("old-bool", Value::Bool(true), &calls));
    not_condition.set_item(Probe::success("bool", Value::Bool(false), &calls));
    assert_eq!(not_condition.get_condition_type(), ConditionTypeEnum::Not);
    assert_eq!(not_condition.get_item().id(), "bool");
    assert!(!not_condition.get_item_result_meta_value(&frame));
    assert_eq!(
        not_condition
            .execute_condition(&ctx, &frame)
            .await
            .expect("NOT 应成功执行"),
        Value::Bool(true)
    );
    assert!(not_condition.get_item_result_meta_value(&frame));

    let body = Probe::success("for-body", Value::Null, &calls);
    let mut for_condition = ForCondition::with_count(99, false, Arc::clone(&body), None);
    for_condition.set_for_node(Probe::success("for-count", json!(2), &calls));
    assert_eq!(for_condition.get_condition_type(), ConditionTypeEnum::For);
    assert_eq!(
        for_condition
            .get_for_node()
            .expect("动态 FOR 节点应已设置")
            .id(),
        "for-count"
    );
    for_condition
        .execute_condition(&ctx, &frame)
        .await
        .expect("FOR 应执行两次");

    let mut iterator_condition = IteratorCondition::new(
        Probe::success("old-iterator", json!([]), &calls),
        false,
        Probe::success("iterator-body", Value::Null, &calls),
        None,
    );
    iterator_condition.set_iterator_node(Probe::success(
        "iterator",
        json!(["first", "second"]),
        &calls,
    ));
    assert_eq!(
        iterator_condition.get_condition_type(),
        ConditionTypeEnum::Iterator
    );
    assert_eq!(iterator_condition.get_iterator_node().id(), "iterator");
    iterator_condition
        .execute_condition(&ctx, &frame)
        .await
        .expect("ITERATOR 应执行两次");

    let pre = Arc::new(PreCondition::new(Probe::success(
        "pre-body",
        Value::Null,
        &calls,
    )));
    let finally = Arc::new(FinallyCondition::new(Probe::success(
        "finally-body",
        Value::Null,
        &calls,
    )));
    assert_eq!(pre.get_condition_type(), ConditionTypeEnum::Pre);
    assert_eq!(finally.get_condition_type(), ConditionTypeEnum::Finally);
    pre.execute_condition(&ctx, &frame)
        .await
        .expect("PRE 应执行真实主体");
    finally
        .execute_condition(&ctx, &frame)
        .await
        .expect("FINALLY 应执行真实主体");

    let mut then_condition = ThenCondition::new();
    then_condition.add_pre_condition(pre);
    then_condition.add_executable(Probe::success("main-body", Value::Null, &calls));
    then_condition.add_finally_condition(finally);
    assert_eq!(then_condition.get_condition_type(), ConditionTypeEnum::Then);
    assert_eq!(then_condition.get_pre_condition_list().len(), 1);
    assert_eq!(then_condition.get_finally_condition_list().len(), 1);
    then_condition
        .execute_condition(&ctx, &frame)
        .await
        .expect("THEN 应按 pre-main-finally 执行");

    let calls = calls.lock().expect("调用记录锁不应中毒");
    assert_eq!(calls.iter().filter(|id| **id == "for-body").count(), 2);
    assert_eq!(calls.iter().filter(|id| **id == "iterator-body").count(), 2);
    assert!(
        calls
            .windows(3)
            .any(|window| { window == ["pre-body", "main-body", "finally-body"] })
    );
}

#[tokio::test]
async fn pre_and_finally_preserve_full_java_executable_lists() {
    let calls = Arc::new(Mutex::new(Vec::new()));
    let (ctx, frame) = execution_context();

    let mut pre = PreCondition::new(Probe::success("pre-old", Value::Null, &calls));
    Condition::set_executable_list(
        &mut pre,
        vec![
            Probe::success("pre-first", Value::Null, &calls),
            Probe::success("pre-second", Value::Null, &calls),
        ],
    );
    assert_eq!(
        Condition::get_executable_list(&pre, "DEFAULT_KEY")
            .iter()
            .map(|item| item.id())
            .collect::<Vec<_>>(),
        ["pre-first", "pre-second"]
    );
    pre.execute_condition(&ctx, &frame)
        .await
        .expect("PRE 应顺序执行完整列表");

    let mut finally = FinallyCondition::new(Probe::success("finally-old", Value::Null, &calls));
    Condition::set_executable_list(
        &mut finally,
        vec![
            Probe::success("finally-first", Value::Null, &calls),
            Probe::success("finally-second", Value::Null, &calls),
        ],
    );
    finally
        .execute_condition(&ctx, &frame)
        .await
        .expect("FINALLY 应顺序执行完整列表");
    assert_eq!(
        *calls.lock().expect("调用记录锁不应中毒"),
        ["pre-first", "pre-second", "finally-first", "finally-second"]
    );

    // Java setExecutableList 允许替换为空列表；空 PRE/FINALLY 是成功的 no-op。
    Condition::set_executable_list(&mut pre, Vec::new());
    assert!(Condition::get_executable_list(&pre, "DEFAULT_KEY").is_empty());
    pre.execute_condition(&ctx, &frame)
        .await
        .expect("空 PRE 列表应成功完成");

    Condition::set_executable_list(
        &mut finally,
        vec![
            Probe::failure("finally-failed", &calls),
            Probe::success("finally-never", Value::Null, &calls),
        ],
    );
    assert!(finally.execute_condition(&ctx, &frame).await.is_err());
    assert!(
        !calls
            .lock()
            .expect("调用记录锁不应中毒")
            .contains(&"finally-never")
    );
}

#[tokio::test]
async fn when_java_named_state_controls_the_real_parallel_executor() {
    let calls = Arc::new(Mutex::new(Vec::new()));
    let (ctx, frame) = execution_context();
    let mut when_condition = WhenCondition::new(vec![
        Probe::success("parallel-a", Value::Null, &calls),
        Probe::success("parallel-b", Value::Null, &calls),
    ]);

    assert_eq!(when_condition.get_condition_type(), ConditionTypeEnum::When);
    assert!(!when_condition.is_ignore_error());
    assert_eq!(when_condition.get_group(), "default");
    assert_eq!(
        when_condition.get_parallel_strategy(),
        ParallelStrategyEnum::All
    );

    when_condition.set_ignore_error(true);
    when_condition.set_group("legacy-group");
    when_condition.set_parallel_strategy(ParallelStrategyEnum::Any);
    when_condition.set_specify_id_set(HashSet::from(["parallel-a".to_string()]));
    when_condition
        .set_thread_executor_class(
            "com.yomahub.liteflow.thread.LiteFlowDefaultGlobalExecutorBuilder",
        )
        .expect("默认全局执行器应可在 setter 阶段预创建");
    when_condition.set_max_wait_time_unit(TimeUnit::Seconds);
    when_condition.set_max_wait_time(2);
    when_condition.set_percentage(0.5);

    assert!(when_condition.is_ignore_error());
    assert_eq!(when_condition.get_group(), "legacy-group");
    assert_eq!(
        when_condition.get_parallel_strategy(),
        ParallelStrategyEnum::Any
    );
    assert_eq!(
        when_condition.get_specify_id_set(),
        HashSet::from(["parallel-a".to_string()])
    );
    assert_eq!(
        when_condition.get_thread_executor_class(),
        Some("com.yomahub.liteflow.thread.LiteFlowDefaultGlobalExecutorBuilder")
    );
    assert_eq!(when_condition.get_max_wait_time(), Some(2));
    assert_eq!(when_condition.get_max_wait_time_unit(), TimeUnit::Seconds);
    assert_eq!(when_condition.get_percentage(), Some(0.5));

    when_condition
        .execute_condition(&ctx, &frame)
        .await
        .expect("WHEN 应通过真实并行执行器完成");
    assert!(!calls.lock().expect("调用记录锁不应中毒").is_empty());
}

#[tokio::test]
async fn and_or_and_while_java_named_methods_preserve_cached_results() {
    let calls = Arc::new(Mutex::new(Vec::new()));
    let (ctx, frame) = execution_context();

    let mut and_or_condition = AndOrCondition::new(BooleanConditionTypeEnum::And, Vec::new());
    and_or_condition.add_item(Probe::success("false-item", Value::Bool(false), &calls));
    and_or_condition.add_item(Probe::success("true-item", Value::Bool(true), &calls));
    and_or_condition.set_boolean_condition_type(BooleanConditionTypeEnum::Or);
    assert_eq!(
        and_or_condition.get_condition_type(),
        ConditionTypeEnum::AndOr
    );
    assert_eq!(
        and_or_condition.get_boolean_condition_type(),
        BooleanConditionTypeEnum::Or
    );
    assert_eq!(and_or_condition.get_item().len(), 2);
    assert!(
        and_or_condition
            .and_or_condition_predicate(&ctx, &frame)
            .test(and_or_condition.get_item()[1].as_ref())
            .await
            .expect("内部谓词应执行布尔项")
    );
    assert_eq!(
        and_or_condition
            .execute_condition(&ctx, &frame)
            .await
            .expect("OR 应成功执行"),
        Value::Bool(true)
    );
    assert_eq!(
        and_or_condition.get_item_result_meta_value(&frame),
        Some(true)
    );

    let empty = AndOrCondition::new(BooleanConditionTypeEnum::And, Vec::new());
    assert!(matches!(
        empty.execute_condition(&ctx, &frame).await,
        Err(LiteflowError::AndOrCondition(message)) if message == "boolean item list is null"
    ));

    let mut while_condition = WhileCondition::new(
        Probe::success("old-while", Value::Bool(false), &calls),
        false,
        Probe::success("while-body", Value::Null, &calls),
        None,
    );
    while_condition.set_while_item(SequenceProbe::new(
        "while-check",
        [Value::Bool(true), Value::Bool(false)],
        &calls,
    ));
    assert_eq!(
        while_condition.get_condition_type(),
        ConditionTypeEnum::While
    );
    assert_eq!(while_condition.get_while_item().id(), "while-check");
    while_condition
        .execute_condition(&ctx, &frame)
        .await
        .expect("WHILE 应执行一次循环体后退出");

    let calls = calls.lock().expect("调用记录锁不应中毒");
    assert_eq!(calls.iter().filter(|id| **id == "while-check").count(), 2);
    assert_eq!(calls.iter().filter(|id| **id == "while-body").count(), 1);
}
