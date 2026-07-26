//! S3 builder/el/operator 验收测试。
//!
//! 覆盖 Java v2.16 的 34 个具体操作符，确保每个独立文件都真实参与
//! AST 构建或运行时执行，而不是只有文件壳。

use liteflow_core::el::{El, parse_el};
use liteflow_core::{FlowBus, LiteflowError, cmp};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

#[test]
fn then_operator_builds_serial_ast() {
    assert!(matches!(parse_el("THEN(a,b)").unwrap(), El::Then(items) if items.len() == 2));
    assert!(matches!(parse_el("SER(a,b)").unwrap(), El::Then(items) if items.len() == 2));
}

#[test]
fn when_operator_builds_parallel_ast() {
    assert!(matches!(
        parse_el("WHEN(a,b)").unwrap(),
        El::When { items, .. } if items.len() == 2
    ));
    assert!(matches!(
        parse_el("PAR(a,b)").unwrap(),
        El::When { items, .. } if items.len() == 2
    ));
}

#[test]
fn and_operator_keeps_single_item_compatibility() {
    assert!(matches!(parse_el("AND(a,b)").unwrap(), El::And(items) if items.len() == 2));
    assert!(matches!(parse_el("AND(a)").unwrap(), El::And(items) if items.len() == 1));
}

#[test]
fn or_operator_keeps_single_item_compatibility() {
    assert!(matches!(parse_el("OR(a,b)").unwrap(), El::Or(items) if items.len() == 2));
    assert!(matches!(parse_el("OR(a)").unwrap(), El::Or(items) if items.len() == 1));
}

#[test]
fn not_operator_requires_exactly_one_item() {
    assert!(matches!(parse_el("NOT(a)").unwrap(), El::Not(_)));
    assert!(parse_el("NOT(a,b)").is_err());
}

#[test]
fn catch_operator_requires_exactly_one_item() {
    assert!(matches!(parse_el("CATCH(a)").unwrap(), El::Catch { .. }));
    assert!(parse_el("CATCH(a,b)").is_err());
}

#[test]
fn node_operator_requires_one_string_id() {
    assert!(matches!(
        parse_el("NODE(\"a\")").unwrap(),
        El::Node(node) if node.id == "a"
    ));
    assert!(parse_el("NODE(a)").is_err());
}

#[test]
fn any_operator_only_accepts_when_caller() {
    assert!(matches!(
        parse_el("WHEN(a,b).any(true)").unwrap(),
        El::When { opts, .. } if opts.any
    ));
    assert!(parse_el("THEN(a).any(true)").is_err());
}

#[test]
fn must_operator_accepts_string_and_node_arguments() {
    assert!(matches!(
        parse_el("WHEN(a,b).must(\"a\", b)").unwrap(),
        El::When { opts, .. } if opts.must == vec!["a".to_string(), "b".to_string()]
    ));
    assert!(parse_el("WHEN(a,b).must()").is_err());
}

#[test]
fn percentage_operator_validates_threshold_range() {
    assert!(matches!(
        parse_el("WHEN(a,b).percentage(0.5)").unwrap(),
        El::When { opts, .. } if opts.percentage == Some(0.5)
    ));
    assert!(parse_el("WHEN(a,b).percentage(1.1)").is_err());
}

#[test]
fn ignore_error_operator_supports_when_and_common_conditions() {
    assert!(matches!(
        parse_el("WHEN(a,b).ignoreError(true)").unwrap(),
        El::When { opts, .. } if opts.ignore_error
    ));
    assert!(matches!(
        parse_el("THEN(a).ignoreError(true)").unwrap(),
        El::Mods(_, mods) if mods.ignore_error
    ));
}

#[test]
fn max_wait_operator_keeps_finally_outside_timeout() {
    let expression = parse_el("THEN(PRE(a),b,FINALLY(c)).maxWaitSeconds(2)").unwrap();
    match expression {
        El::Then(items) => {
            assert_eq!(items.len(), 2);
            assert!(matches!(
                &items[0],
                El::Mods(_, mods) if mods.max_wait_ms == Some(2_000)
            ));
            assert!(matches!(&items[1], El::Fin(_)));
        }
        other => panic!("应重写为外层 THEN，实际为 {other:?}"),
    }
    assert!(parse_el("FINALLY(a).maxWaitMilliseconds(10)").is_err());
}

#[test]
fn retry_operator_records_exception_filters() {
    assert!(matches!(
        parse_el("a.retry(2,\"ParseException\")").unwrap(),
        El::Mods(_, mods)
            if mods.retry == Some(2)
                && mods.retry_for == vec!["ParseException".to_string()]
    ));
}

#[tokio::test]
async fn retry_operator_filters_real_runtime_errors() {
    let bus = FlowBus::new();
    let matched_count = Arc::new(AtomicUsize::new(0));
    let matched_counter = Arc::clone(&matched_count);
    bus.register(
        "matched",
        cmp(move |_| {
            let matched_counter = Arc::clone(&matched_counter);
            async move {
                matched_counter.fetch_add(1, Ordering::SeqCst);
                Err(LiteflowError::Parse("retry me".to_string()))
            }
        }),
    );
    let unmatched_count = Arc::new(AtomicUsize::new(0));
    let unmatched_counter = Arc::clone(&unmatched_count);
    bus.register(
        "unmatched",
        cmp(move |_| {
            let unmatched_counter = Arc::clone(&unmatched_counter);
            async move {
                unmatched_counter.fetch_add(1, Ordering::SeqCst);
                Err(LiteflowError::Parse("do not retry".to_string()))
            }
        }),
    );

    let matched = bus
        .execute_with_el("matched.retry(2,\"ParseException\")")
        .await;
    let unmatched = bus
        .execute_with_el("unmatched.retry(2,\"NodeBuildException\")")
        .await;

    assert!(!matched.is_success());
    assert!(!unmatched.is_success());
    assert_eq!(matched_count.load(Ordering::SeqCst), 3);
    assert_eq!(unmatched_count.load(Ordering::SeqCst), 1);
}

#[test]
fn elif_and_else_operators_extend_if_ast() {
    assert!(matches!(
        parse_el("IF(check,a).ELIF(other,b).ELSE(c)").unwrap(),
        El::If { elifs, els, .. } if elifs.len() == 1 && els.is_some()
    ));
    assert!(parse_el("THEN(a).ELSE(b)").is_err());
}

#[test]
fn to_and_default_operators_extend_switch_ast() {
    assert!(matches!(
        parse_el("SWITCH(selector).TO(\"a:t1\",b).DEFAULT(c)").unwrap(),
        El::Switch { targets, default, .. }
            if targets.len() == 2 && default.is_some()
    ));
    assert!(parse_el("THEN(a).TO(b)").is_err());
}

#[test]
fn do_break_and_parallel_operators_extend_loop_ast() {
    assert!(matches!(
        parse_el("FOR(3).parallel(true).DO(work).BREAK(stop)").unwrap(),
        El::ForCount {
            count: 3,
            parallel: Some(_),
            brk: Some(_),
            ..
        }
    ));
    assert!(parse_el("THEN(a).BREAK(stop)").is_err());
}

#[test]
fn primary_control_flow_operators_build_typed_ast() {
    assert!(matches!(
        parse_el("IF(check,a,b)").unwrap(),
        El::If { els: Some(_), .. }
    ));
    assert!(matches!(
        parse_el("SWITCH(selector)").unwrap(),
        El::Switch { targets, .. } if targets.is_empty()
    ));
    assert!(matches!(parse_el("FOR(counter)").unwrap(), El::For { .. }));
    assert!(matches!(
        parse_el("FOR(4)").unwrap(),
        El::ForCount { count: 4, .. }
    ));
    assert!(matches!(
        parse_el("WHILE(false)").unwrap(),
        El::While { node, .. } if matches!(*node, El::Boolean(false))
    ));
    assert!(matches!(
        parse_el("ITERATOR(items)").unwrap(),
        El::Iter { .. }
    ));
    assert!(parse_el("IF(check)").is_err());
    assert!(parse_el("SWITCH(a,b)").is_err());
    assert!(parse_el("FOR(1.5)").is_err());
}

#[test]
fn pre_and_finally_operators_reject_empty_bodies() {
    assert!(matches!(parse_el("PRE(a,b)").unwrap(), El::Pre(_)));
    assert!(matches!(parse_el("FINALLY(a,b)").unwrap(), El::Fin(_)));
    assert!(parse_el("PRE()").is_err());
    assert!(parse_el("FINALLY()").is_err());
}

#[test]
fn id_tag_and_data_operators_preserve_java_scope_semantics() {
    assert!(matches!(
        parse_el("THEN(a).id(\"condition-1\").tag(\"audit\")").unwrap(),
        El::Mods(_, mods)
            if mods.id.as_deref() == Some("condition-1")
                && mods.tag.as_deref() == Some("audit")
    ));
    assert!(matches!(
        parse_el("a.id(\"instance-a\").tag(\"blue\")").unwrap(),
        El::Node(node)
            if node.alias.as_deref() == Some("instance-a")
                && node.tag.as_deref() == Some("blue")
    ));
    let expression = parse_el("THEN(a,IF(check,b,c)).data(\"payload\")").unwrap();
    match expression {
        El::Then(items) => {
            assert!(matches!(&items[0], El::Node(node) if node.data.as_deref() == Some("payload")));
            assert!(matches!(
                &items[1],
                El::If { cond, then, els: Some(els), .. }
                    if matches!(cond.as_ref(), El::Node(node) if node.data.as_deref() == Some("payload"))
                        && matches!(then.as_ref(), El::Node(node) if node.data.as_deref() == Some("payload"))
                        && matches!(els.as_ref(), El::Node(node) if node.data.as_deref() == Some("payload"))
            ));
        }
        other => panic!("DATA 应递归作用于 THEN 中全部节点，实际为 {other:?}"),
    }
}

#[test]
fn bind_and_thread_pool_operators_cover_node_condition_when_and_loop() {
    assert!(matches!(
        parse_el("a.bind(\"tenant\",\"t1\",true)").unwrap(),
        El::Node(node)
            if node.bind == vec![("tenant".to_string(), "t1".to_string())]
                && node.bind_override
    ));
    assert!(matches!(
        parse_el("THEN(a).bind(\"tenant\",\"t1\",true)").unwrap(),
        El::Mods(_, mods)
            if mods.bind == vec![("tenant".to_string(), "t1".to_string())]
                && mods.bind_override
    ));
    assert!(matches!(
        parse_el("WHEN(a,b).threadPool(\"fast\")").unwrap(),
        El::When { opts, .. } if opts.thread_pool.as_deref() == Some("fast")
    ));
    assert!(matches!(
        parse_el("FOR(2).DO(a).threadPool(\"loop\")").unwrap(),
        El::Mods(inner, mods)
            if matches!(*inner, El::ForCount { .. })
                && mods.thread_pool.as_deref() == Some("loop")
    ));
    assert!(parse_el("THEN(a).threadPool(\"wrong\")").is_err());
}

#[tokio::test]
async fn while_boolean_literal_operator_executes_without_registered_node() {
    let bus = FlowBus::new();
    bus.register("body", cmp(|_| async { Ok(serde_json::Value::Null) }));
    let response = bus.execute_with_el("WHILE(false).DO(body)").await;
    assert!(response.is_success(), "{:?}", response.cause);
}
