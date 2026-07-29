//! `IfCondition` 对 Java v2.16.0 的对象级语义测试。

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use liteflow_core::exception::{LFResult, LiteflowError};
use liteflow_core::flow::element::condition::Condition;
use liteflow_core::flow::element::condition::if_condition::IfCondition;
use liteflow_core::flow::element::condition::pre_condition::PreCondition;
use liteflow_core::flow::element::executable::Executable;
use liteflow_core::slot::{Ctx, Frame, Slot};
use liteflow_core::{ConditionTypeEnum, ExecuteableTypeEnum};
use serde_json::Value;

struct Probe {
    id: &'static str,
    output: Value,
    accessible: bool,
    execute_calls: Arc<AtomicUsize>,
    access_calls: Arc<AtomicUsize>,
}

#[async_trait::async_trait]
impl Executable for Probe {
    async fn execute(&self, _ctx: &Ctx, _frame: &Frame) -> LFResult<Value> {
        self.execute_calls.fetch_add(1, Ordering::SeqCst);
        Ok(self.output.clone())
    }

    async fn is_access(&self, _ctx: &Ctx, _frame: &Frame) -> bool {
        self.access_calls.fetch_add(1, Ordering::SeqCst);
        self.accessible
    }

    fn execute_type(&self) -> ExecuteableTypeEnum {
        ExecuteableTypeEnum::Node
    }

    fn id(&self) -> &str {
        self.id
    }
}

fn probe(id: &'static str, output: Value, execute_calls: &Arc<AtomicUsize>) -> Arc<dyn Executable> {
    Arc::new(Probe {
        id,
        output,
        accessible: true,
        execute_calls: Arc::clone(execute_calls),
        access_calls: Arc::new(AtomicUsize::new(0)),
    })
}

fn access_probe(
    id: &'static str,
    output: Value,
    accessible: bool,
    execute_calls: &Arc<AtomicUsize>,
    access_calls: &Arc<AtomicUsize>,
) -> Arc<dyn Executable> {
    Arc::new(Probe {
        id,
        output,
        accessible,
        execute_calls: Arc::clone(execute_calls),
        access_calls: Arc::clone(access_calls),
    })
}

fn context() -> Ctx {
    Ctx::new(Arc::new(Slot::new(
        "if-condition".to_string(),
        "if-chain",
        Value::Null,
    )))
}

#[tokio::test]
async fn if_condition_executes_true_elif_false_and_empty_false_paths() {
    let calls = Arc::new(AtomicUsize::new(0));
    let true_condition = IfCondition::new(
        probe("if", Value::Bool(true), &calls),
        probe("true", Value::String("true-result".into()), &calls),
        vec![(
            probe("elif", Value::Bool(true), &calls),
            probe("elif-target", Value::String("elif-result".into()), &calls),
        )],
        Some(probe("false", Value::String("false-result".into()), &calls)),
    );
    assert_eq!(
        true_condition
            .execute_condition(&context(), &Frame::root())
            .await
            .expect("IF true 分支应执行"),
        Value::String("true-result".to_string())
    );

    let elif_condition = IfCondition::new(
        probe("if", Value::Bool(false), &calls),
        probe("true", Value::Null, &calls),
        vec![
            (
                probe("elif-false", Value::Bool(false), &calls),
                probe("unused-elif", Value::Null, &calls),
            ),
            (
                probe("elif-true", Value::Bool(true), &calls),
                probe("elif-target", Value::String("elif-result".into()), &calls),
            ),
        ],
        Some(probe("false", Value::Null, &calls)),
    );
    assert_eq!(
        elif_condition
            .execute(&context(), &Frame::root())
            .await
            .expect("第二个 ELIF 应命中"),
        Value::String("elif-result".to_string())
    );

    let false_condition = IfCondition::new(
        probe("if", Value::Bool(false), &calls),
        probe("true", Value::Null, &calls),
        Vec::new(),
        Some(probe("false", Value::String("false-result".into()), &calls)),
    );
    assert_eq!(
        false_condition
            .execute(&context(), &Frame::root())
            .await
            .expect("ELSE 应执行"),
        Value::String("false-result".to_string())
    );

    let no_false = IfCondition::new(
        probe("if", Value::Bool(false), &calls),
        probe("true", Value::Null, &calls),
        Vec::new(),
        None,
    );
    assert_eq!(
        no_false
            .execute(&context(), &Frame::root())
            .await
            .expect("Java 允许 IF 没有 ELSE"),
        Value::Null
    );
}

#[tokio::test]
async fn if_condition_preserves_missing_true_type_and_target_errors() {
    let calls = Arc::new(AtomicUsize::new(0));
    let mut missing_true = IfCondition::new(
        probe("if", Value::Bool(true), &calls),
        probe("old-true", Value::Null, &calls),
        Vec::new(),
        None,
    );
    missing_true.replace_executable_group("IF_TRUE_CASE_KEY", Vec::new());
    assert!(missing_true.get_true_case_executable_item().is_none());
    assert!(matches!(
        missing_true.execute(&context(), &Frame::root()).await,
        Err(LiteflowError::NoIfTrueNode(node)) if node == "if"
    ));

    let invalid_if = IfCondition::new(
        probe("if", Value::String("true".into()), &calls),
        probe("true", Value::Null, &calls),
        Vec::new(),
        None,
    );
    assert!(matches!(
        invalid_if.execute(&context(), &Frame::root()).await,
        Err(LiteflowError::NodeTypeError { expect, .. }) if expect == "boolean"
    ));

    let invalid_elif = IfCondition::new(
        probe("if", Value::Bool(false), &calls),
        probe("true", Value::Null, &calls),
        vec![(
            probe("elif", Value::Number(1.into()), &calls),
            probe("elif-target", Value::Null, &calls),
        )],
        None,
    );
    assert!(matches!(
        invalid_elif.execute(&context(), &Frame::root()).await,
        Err(LiteflowError::NodeTypeError { expect, .. }) if expect == "boolean"
    ));

    let pre: Arc<dyn Executable> =
        Arc::new(PreCondition::new(probe("inside-pre", Value::Null, &calls)));
    for condition in [
        IfCondition::new(
            probe("if", Value::Bool(true), &calls),
            Arc::clone(&pre),
            Vec::new(),
            None,
        ),
        IfCondition::new(
            probe("if", Value::Bool(false), &calls),
            probe("true", Value::Null, &calls),
            Vec::new(),
            Some(Arc::clone(&pre)),
        ),
        IfCondition::new(
            probe("if", Value::Bool(false), &calls),
            probe("true", Value::Null, &calls),
            vec![(probe("elif", Value::Bool(true), &calls), Arc::clone(&pre))],
            None,
        ),
    ] {
        assert!(matches!(
            condition.execute(&context(), &Frame::root()).await,
            Err(LiteflowError::TargetCannotBePreOrFinally(_))
        ));
    }
}

#[tokio::test]
async fn inaccessible_if_or_elif_stops_the_java_nested_condition() {
    let execute_calls = Arc::new(AtomicUsize::new(0));
    let access_calls = Arc::new(AtomicUsize::new(0));
    let false_calls = Arc::new(AtomicUsize::new(0));
    let denied_if = IfCondition::new(
        access_probe(
            "if",
            Value::Bool(true),
            false,
            &execute_calls,
            &access_calls,
        ),
        probe("true", Value::Null, &false_calls),
        Vec::new(),
        None,
    );
    denied_if
        .execute(&context(), &Frame::root())
        .await
        .expect("isAccess=false 应跳过整个 IF");
    assert_eq!(access_calls.load(Ordering::SeqCst), 1);
    assert_eq!(execute_calls.load(Ordering::SeqCst), 0);
    assert_eq!(false_calls.load(Ordering::SeqCst), 0);

    let elif_access_calls = Arc::new(AtomicUsize::new(0));
    let elif_execute_calls = Arc::new(AtomicUsize::new(0));
    let denied_elif = IfCondition::new(
        probe("if", Value::Bool(false), &execute_calls),
        probe("true", Value::Null, &false_calls),
        vec![(
            access_probe(
                "elif",
                Value::Bool(true),
                false,
                &elif_execute_calls,
                &elif_access_calls,
            ),
            probe("elif-target", Value::Null, &false_calls),
        )],
        Some(probe(
            "false",
            Value::String("must-not-run".into()),
            &false_calls,
        )),
    );
    assert_eq!(
        denied_elif
            .execute(&context(), &Frame::root())
            .await
            .expect("嵌套 ELIF 不可访问时应结束该内层 IF"),
        Value::Null
    );
    assert_eq!(elif_access_calls.load(Ordering::SeqCst), 1);
    assert_eq!(elif_execute_calls.load(Ordering::SeqCst), 0);
    assert_eq!(false_calls.load(Ordering::SeqCst), 0);
}

#[test]
fn if_condition_java_state_and_typed_groups_share_one_source() {
    let calls = Arc::new(AtomicUsize::new(0));
    let mut condition = IfCondition::new(
        probe("if-a", Value::Bool(false), &calls),
        probe("true-a", Value::Null, &calls),
        vec![(
            probe("elif-a", Value::Bool(false), &calls),
            probe("elif-target-a", Value::Null, &calls),
        )],
        None,
    );

    assert_eq!(condition.get_condition_type(), ConditionTypeEnum::If);
    assert_eq!(condition.condition_type(), ConditionTypeEnum::If);
    assert_eq!(
        <IfCondition as Condition>::condition_type(&condition),
        ConditionTypeEnum::If
    );
    condition.set_if_item(probe("if-b", Value::Bool(false), &calls));
    condition.set_true_case_executable_item(probe("true-b", Value::Null, &calls));
    condition.set_false_case_executable_item(probe("false-b", Value::Null, &calls));
    assert_eq!(condition.get_if_item().id(), "if-b");
    assert_eq!(
        condition
            .get_true_case_executable_item()
            .expect("应存在 true 分支")
            .id(),
        "true-b"
    );
    assert_eq!(
        condition
            .get_false_case_executable_item()
            .expect("应存在 false 分支")
            .id(),
        "false-b"
    );

    condition.replace_executable_group(
        "IF_KEY",
        vec![
            probe("if-c", Value::Bool(false), &calls),
            probe("elif-c", Value::Bool(false), &calls),
        ],
    );
    condition.replace_executable_group(
        "IF_TRUE_CASE_KEY",
        vec![
            probe("true-c", Value::Null, &calls),
            probe("elif-target-c", Value::Null, &calls),
        ],
    );
    condition.replace_executable_group("IF_FALSE_CASE_KEY", Vec::new());
    condition.replace_executable_group("CUSTOM_KEY", vec![probe("custom", Value::Null, &calls)]);
    let groups = condition.get_executable_group();
    assert_eq!(groups["IF_KEY"][0].id(), "if-c");
    assert_eq!(groups["IF_KEY"][1].id(), "elif-c");
    assert_eq!(groups["IF_TRUE_CASE_KEY"][0].id(), "true-c");
    assert_eq!(groups["IF_TRUE_CASE_KEY"][1].id(), "elif-target-c");
    assert!(!groups.contains_key("IF_FALSE_CASE_KEY"));
    assert_eq!(groups["CUSTOM_KEY"][0].id(), "custom");

    Condition::set_id(&mut condition, "if-condition");
    assert_eq!(Condition::get_id(&condition), "if-condition");
    let mut node_ids = Executable::collect_node_ids(&condition);
    node_ids.sort();
    assert_eq!(
        node_ids,
        vec!["custom", "elif-c", "elif-target-c", "if-c", "true-c"]
    );
    Executable::apply_chain_cmp_data(&condition, "payload");
    assert_eq!(Executable::id(&condition), "IF");
}
