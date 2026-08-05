//! Condition trait 分组协议遍历测试（批次 J）。
//!
//! 对应 Java: `Condition#getExecutableGroup/setExecutableGroup`、
//! `ConditionKey` 常量分组与 `LiteflowMetaOperator#getNodes` 的递归入口。
//! 对全部 Condition 类型验证 `typed_executable_group` /
//! `replace_typed_executable_group` / `collect_node_ids` /
//! `apply_chain_cmp_data` / `condition_base` / `condition_type` 协议。

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
use liteflow_core::flow::element::condition::when_condition::WhenCondition;
use liteflow_core::flow::element::condition::while_condition::WhileCondition;
use liteflow_core::flow::element::condition::{BooleanConditionTypeEnum, Condition};
use liteflow_core::flow::element::executable::Executable;
use liteflow_core::{ConditionTypeEnum, Ctx, Frame, NodeRef, Slot, cmp};
use serde_json::{Value, json};
use std::sync::Arc;

fn node(id: &str) -> Arc<dyn Executable> {
    let owned = id.to_string();
    let component: Arc<dyn liteflow_core::NodeComponent> = Arc::new(cmp(move |_| {
        let owned = owned.clone();
        async move { Ok(json!({"id": owned})) }
    }));
    Arc::new(liteflow_core::flow::element::node::Node::new(
        NodeRef::new(id),
        component,
    ))
}

fn ctx() -> Ctx {
    Ctx::new(Arc::new(Slot::new(
        "RID-PROTO".to_string(),
        "main",
        Value::Null,
    )))
}

/// 通用协议断言：分组往返、节点收集与条件类型。
fn assert_protocol(condition: &mut dyn Condition, expected_type: ConditionTypeEnum) {
    assert_eq!(condition.condition_type(), expected_type);
    let _base = condition.condition_base();
    let groups = condition.typed_executable_group();
    assert!(!groups.is_empty());
    for (key, members) in &groups {
        // 替换回同一组后组内容不变
        let replaced = members.clone();
        let ok = condition.replace_typed_executable_group(key, replaced.clone());
        assert!(ok, "group key {key} 应可替换");
        let after = condition.typed_executable_group();
        let after_list = after.get(key).expect("替换后组仍存在");
        assert_eq!(after_list.len(), replaced.len());
    }
    // 未知分组键拒绝
    assert!(!condition.replace_typed_executable_group("NO_SUCH_KEY", Vec::new()));
    // 空列表替换按各 Condition 自身实现处理，不在此处做统一断言
    // 节点收集不为空
    let ids = condition.collect_node_ids();
    assert!(!ids.is_empty());
    // 组件数据传播不 panic
    condition.apply_chain_cmp_data("chain-data");
    let _ = ctx();
}

#[test]
fn then_condition_protocol() {
    let mut condition = ThenCondition::new();
    condition.add_executable(node("a"));
    condition.add_executable(node("b"));
    assert_protocol(&mut condition, ConditionTypeEnum::Then);
}

#[test]
fn when_condition_protocol() {
    let mut condition = WhenCondition::new(vec![node("a"), node("b")]);
    assert_protocol(&mut condition, ConditionTypeEnum::When);
}

#[test]
fn if_condition_protocol() {
    let mut condition = IfCondition::new(
        node("cond"),
        node("true_case"),
        vec![(node("elif_cond"), node("elif_case"))],
        Some(node("false_case")),
    );
    assert_protocol(&mut condition, ConditionTypeEnum::If);
}

#[test]
fn switch_condition_protocol() {
    let mut condition =
        SwitchCondition::new(node("sw"), vec![node("ta"), node("tb")], Some(node("td")));
    assert_protocol(&mut condition, ConditionTypeEnum::Switch);
}

#[test]
fn for_condition_protocol() {
    let mut condition = ForCondition::new(node("counter"), false, node("body"), Some(node("brk")));
    assert_protocol(&mut condition, ConditionTypeEnum::For);
}

#[test]
fn while_condition_protocol() {
    let mut condition = WhileCondition::new(node("while_cond"), false, node("body"), None);
    assert_protocol(&mut condition, ConditionTypeEnum::While);
}

#[test]
fn iterator_condition_protocol() {
    let mut condition = IteratorCondition::new(node("iter"), false, node("body"), None);
    assert_protocol(&mut condition, ConditionTypeEnum::Iterator);
}

#[test]
fn catch_condition_protocol() {
    let mut condition = CatchCondition::new(node("caught"), Some(node("handler")));
    assert_protocol(&mut condition, ConditionTypeEnum::Catch);
}

#[test]
fn and_or_condition_protocol() {
    let mut condition =
        AndOrCondition::new(BooleanConditionTypeEnum::And, vec![node("a"), node("b")]);
    assert_protocol(&mut condition, ConditionTypeEnum::AndOr);
}

#[test]
fn not_condition_protocol() {
    let mut condition = NotCondition::new(node("inner"));
    assert_protocol(&mut condition, ConditionTypeEnum::Not);
}

#[test]
fn pre_and_finally_condition_protocol() {
    let mut pre = PreCondition::new(node("pre"));
    assert_protocol(&mut pre, ConditionTypeEnum::Pre);
    let mut finally = FinallyCondition::new(node("fin"));
    assert_protocol(&mut finally, ConditionTypeEnum::Finally);
}

/// ThenCondition 的 PRE/FINALLY 分组替换（Java ThenCondition#addExecutable 分流）。
#[test]
fn then_condition_pre_finally_groups() {
    let mut condition = ThenCondition::new();
    let pre = PreCondition::new(node("pre_inner"));
    let body = node("body");
    let fin = FinallyCondition::new(node("fin_inner"));
    condition.add_executable(Arc::new(pre));
    condition.add_executable(Arc::clone(&body));
    condition.add_executable(Arc::new(fin));

    let groups = condition.typed_executable_group();
    assert!(groups.contains_key("PRE_KEY"));
    assert!(groups.contains_key("DEFAULT_KEY"));
    assert!(groups.contains_key("FINALLY_KEY"));
    assert_eq!(groups["DEFAULT_KEY"].len(), 1);
    assert_eq!(groups["DEFAULT_KEY"][0].id(), "body");

    // 替换 PRE 组
    let replacement = node("pre2");
    assert!(condition.replace_typed_executable_group("PRE_KEY", vec![Arc::clone(&replacement)]));
    let groups = condition.typed_executable_group();
    assert_eq!(groups["PRE_KEY"].len(), 1);
    assert_eq!(groups["PRE_KEY"][0].id(), "pre2");
}

/// Frame 无关的 Executable 元数据在协议遍历后可读。
#[tokio::test]
async fn protocol_nodes_keep_executable_metadata() {
    let condition = WhenCondition::new(vec![node("a")]);
    let ids = condition.collect_node_ids();
    assert_eq!(ids, vec!["a".to_string()]);
    let _frame = Frame::root();
}
