use liteflow_core::el::parse_el;
use liteflow_core::{FlowBus, cmp};
use liteflow_el_builder::{ELBuilderError, ELBus, ELWrapper, IntoELWrapper};
use serde_json::json;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

#[test]
fn node_then_and_formatted_output_match_java_shape() {
    let wrapper = ELBus::then(["a", "b"]).expect("THEN 构建应成功");

    assert_eq!(wrapper.to_el().unwrap(), "THEN(a,b);");
    assert_eq!(
        wrapper.to_el_with_format(true).unwrap(),
        "THEN(\n\ta,\n\tb\n);"
    );
}

#[test]
fn common_properties_preserve_data_bind_timeout_and_retry() {
    let wrapper = ELBus::element("a")
        .data("payload", &json!({"x": 1}))
        .unwrap()
        .bind("tenant", "t\"1")
        .max_wait_seconds(3)
        .retry_for(2, ["BizError"]);

    assert_eq!(
        wrapper.to_el().unwrap(),
        "payload = \"{\\\"x\\\":1}\";\na.data(payload).bind(\"tenant\", \"t\\\"1\").maxWaitSeconds(3).retry(2,\"BizError\");"
    );
    assert_eq!(
        wrapper.to_expression().unwrap(),
        "a.data(\"{\\\"x\\\":1}\").bind(\"tenant\", \"t\\\"1\").maxWaitSeconds(3).retry(2)"
    );
}

#[test]
fn boolean_and_non_boolean_positions_are_checked() {
    let boolean = ELBus::and(["a", "b"]).unwrap();
    let error = match ELBus::then([boolean]) {
        Ok(_) => panic!("THEN 不应接受 AND"),
        Err(error) => error,
    };
    assert!(matches!(error, ELBuilderError::InvalidParameter(_)));

    let non_boolean = ELBus::then(["a"]).unwrap();
    let error = match ELBus::not(non_boolean) {
        Ok(_) => panic!("NOT 不应接受 THEN"),
        Err(error) => error,
    };
    assert!(matches!(error, ELBuilderError::InvalidParameter(_)));
}

#[test]
fn if_elif_else_and_switch_are_rendered() {
    let if_wrapper = ELBus::if_then("check_a", "a")
        .unwrap()
        .el_if_opt("check_b", "b")
        .unwrap()
        .else_opt("c")
        .unwrap();
    assert_eq!(
        if_wrapper.to_expression().unwrap(),
        "IF(check_a,a).ELIF(check_b,b).ELSE(c)"
    );

    let switch_wrapper = ELBus::switch_opt("selector")
        .unwrap()
        .to(["a", "b"])
        .unwrap()
        .default_opt("c")
        .unwrap();
    assert_eq!(
        switch_wrapper.to_expression().unwrap(),
        "SWITCH(selector).TO(a,b).DEFAULT(c)"
    );
}

#[test]
fn when_and_par_reject_conflicting_options() {
    let when = ELBus::when(["a", "b"]).unwrap().any(true).percentage(0.5);
    assert!(matches!(
        when.to_expression(),
        Err(ELBuilderError::ConflictingOptions(_))
    ));

    let par = ELBus::par(["a", "b"]).unwrap().any(true).must(["a"]);
    assert!(matches!(
        par.to_expression(),
        Err(ELBuilderError::ConflictingOptions(_))
    ));
}

#[test]
fn loop_catch_pre_finally_and_retry_are_available() {
    let loop_wrapper = ELBus::while_opt("continue")
        .unwrap()
        .parallel(true)
        .do_opt("work")
        .unwrap()
        .break_opt("stop")
        .unwrap();
    assert_eq!(
        loop_wrapper.to_expression().unwrap(),
        "WHILE(node(\"continue\")).parallel(true).DO(work).BREAK(stop)"
    );

    let catch_wrapper = ELBus::catch_exception("danger")
        .unwrap()
        .do_opt("fallback")
        .unwrap();
    assert_eq!(
        catch_wrapper.to_expression().unwrap(),
        "CATCH(danger).DO(fallback)"
    );

    let then_wrapper = ELBus::then(["main"])
        .unwrap()
        .pre(["prepare"])
        .unwrap()
        .finally_opt(["cleanup"])
        .unwrap();
    assert_eq!(
        then_wrapper.to_expression().unwrap(),
        "THEN(PRE(prepare),main,FINALLY(cleanup))"
    );
}

#[test]
fn generated_supported_expressions_round_trip_through_core_parser() {
    let property_expression = ELBus::element("a")
        .data("payload", &json!({"x": 1}))
        .unwrap()
        .bind("tenant", "t1")
        .max_wait_seconds(3)
        .retry(2)
        .to_expression()
        .unwrap();
    let expressions = vec![
        ELBus::then(["a", "b"]).unwrap().to_expression().unwrap(),
        ELBus::ser(["a", "b"]).unwrap().to_expression().unwrap(),
        ELBus::when(["a", "b"])
            .unwrap()
            .any(true)
            .to_expression()
            .unwrap(),
        ELBus::par(["a", "b"])
            .unwrap()
            .ignore_error(true)
            .to_expression()
            .unwrap(),
        ELBus::if_opt("check", "a", "b")
            .unwrap()
            .to_expression()
            .unwrap(),
        ELBus::switch_opt("selector")
            .unwrap()
            .to(["a", "b"])
            .unwrap()
            .default_opt("c")
            .unwrap()
            .to_expression()
            .unwrap(),
        ELBus::catch_exception("danger")
            .unwrap()
            .do_opt("fallback")
            .unwrap()
            .to_expression()
            .unwrap(),
        ELBus::for_opt_count(3)
            .parallel(true)
            .do_opt("work")
            .unwrap()
            .to_expression()
            .unwrap(),
        ELBus::while_opt("continue")
            .unwrap()
            .parallel(true)
            .do_opt("work")
            .unwrap()
            .break_opt("stop")
            .unwrap()
            .to_expression()
            .unwrap(),
        property_expression,
    ];

    for expression in expressions {
        parse_el(&expression).unwrap_or_else(|error| {
            panic!("核心解析器应接受 Builder 输出 `{expression}`: {error}")
        });
    }
}

#[test]
fn node_ids_conflicting_with_qlexpress_tokens_use_explicit_node_operator() {
    let keyword = ELBus::element("continue").to_expression().unwrap();
    let punctuation = ELBus::element("inventory-check").to_expression().unwrap();
    let ordinary = ELBus::element("inventory_check").to_expression().unwrap();
    let java_dollar = ELBus::element("inventory$check").to_expression().unwrap();

    assert_eq!(keyword, r#"node("continue")"#);
    assert_eq!(punctuation, r#"node("inventory-check")"#);
    assert_eq!(ordinary, "inventory_check");
    assert_eq!(java_dollar, "inventory$check");
    parse_el(&keyword).unwrap();
    parse_el(&punctuation).unwrap();
}

#[tokio::test]
async fn fixed_count_loop_generated_by_builder_executes_in_core() {
    let bus = FlowBus::new();
    let executions = Arc::new(AtomicUsize::new(0));
    let counter = Arc::clone(&executions);
    bus.register(
        "work",
        cmp(move |_| {
            let counter = Arc::clone(&counter);
            async move {
                counter.fetch_add(1, Ordering::SeqCst);
                Ok(serde_json::Value::Null)
            }
        }),
    );

    let expression = ELBus::for_opt_count(3)
        .do_opt("work")
        .unwrap()
        .to_expression()
        .unwrap();
    let response = bus.execute_with_el(&expression).await;

    assert!(response.is_success(), "{}", response.message);
    assert_eq!(executions.load(Ordering::SeqCst), 3);
}

#[test]
fn boxed_wrappers_support_heterogeneous_builder_lists() {
    let items = vec![
        ELBus::element("a").into_el_wrapper(),
        ELBus::node("b").into_el_wrapper(),
    ];
    let wrapper = ELBus::then(items).unwrap();
    assert_eq!(wrapper.to_expression().unwrap(), "THEN(a,node(\"b\"))");
}
