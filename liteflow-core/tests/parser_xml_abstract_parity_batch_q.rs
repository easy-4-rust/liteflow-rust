//! ParserHelper XML 特殊输入与抽象链/循环检测补测（批次 Q）。
//!
//! 覆盖：
//! - XML CData/Text/注释/未闭合链的解析分支
//! - 抽象链继承（extends）与实现链的构建
//! - 循环链引用与循环继承检测
//! - 缺失父链的构建错误

use liteflow_core::parser::helper::{ChainDef, ParserHelper, RuleDefinitionPlan};
use liteflow_core::{FlowBus, cmp};
use serde_json::Value;
use std::collections::HashSet;

fn plan_with(chains: Vec<ChainDef>) -> RuleDefinitionPlan {
    let mut plan = RuleDefinitionPlan::new();
    for chain in chains {
        plan.push_chain(chain);
    }
    plan
}

fn chain(id: &str, body: &str) -> ChainDef {
    ChainDef {
        id: id.to_string(),
        namespace: String::new(),
        route: None,
        body: body.to_string(),
        extends: None,
        thread_pool_executor_class: None,
        enable: true,
    }
}

/// 抽象链继承：父链占位符被子链实现替换后执行。
#[test]
fn abstract_chain_inheritance_resolves_and_builds() {
    let bus = FlowBus::new();
    bus.register("a", cmp(|_| async { Ok(Value::Null) }));
    bus.register("b", cmp(|_| async { Ok(Value::Null) }));
    let plan = plan_with(vec![
        ChainDef {
            id: "parent_chain".to_string(),
            namespace: String::new(),
            route: None,
            body: "THEN(a, {{impl}})".to_string(),
            extends: None,
            thread_pool_executor_class: None,
            enable: true,
        },
        ChainDef {
            id: "child_chain".to_string(),
            namespace: String::new(),
            route: None,
            body: "{{impl}} = THEN(b);".to_string(),
            extends: Some("parent_chain".to_string()),
            thread_pool_executor_class: None,
            enable: true,
        },
    ]);
    let built = plan.build_all(&bus).expect("抽象链应构建");
    assert!(built.contains(&"child_chain".to_string()));
}

/// 循环链引用检测。
#[test]
fn cyclic_chain_reference_detected() {
    let bus = FlowBus::new();
    bus.register("a", cmp(|_| async { Ok(Value::Null) }));
    let plan = plan_with(vec![
        chain("cycle_a", "THEN(cycle_b)"),
        chain("cycle_b", "THEN(cycle_a)"),
    ]);
    let error = plan.build_all(&bus).expect_err("循环引用应报错");
    assert!(error.to_string().contains("cyclic"));
}

/// 缺失父链的构建错误。
#[test]
fn missing_parent_chain_rejected() {
    let bus = FlowBus::new();
    bus.register("a", cmp(|_| async { Ok(Value::Null) }));
    let plan = plan_with(vec![ChainDef {
        id: "orphan".to_string(),
        namespace: String::new(),
        route: None,
        body: "{{impl}} = THEN(a);".to_string(),
        extends: Some("missing_parent".to_string()),
        thread_pool_executor_class: None,
        enable: true,
    }]);
    let error = plan.build_all(&bus).expect_err("缺失父链应报错");
    assert!(
        error.to_string().contains("missing_parent") || error.to_string().contains("not found")
    );
}

/// XML 链解析：CData 内的 EL 文本。
#[test]
fn xml_cdata_chain_body_parsed() {
    let bus = FlowBus::new();
    bus.register("a", cmp(|_| async { Ok(Value::Null) }));
    let xml = r#"<flow><chain id="cdata_chain"><![CDATA[THEN(a)]]></chain></flow>"#;
    let mut plan = RuleDefinitionPlan::new();
    let mut ids = HashSet::new();
    ParserHelper::parse_chain_document(&[xml.to_string()], &mut ids, &mut plan)
        .expect("CData 链应解析");
    // chain_id_set 跨文档清空（Java 语义），链定义保留在计划中
    assert_eq!(plan.chain_count(), 1);
    plan.build_all(&bus).expect("构建成功");
    assert!(bus.contains_chain("cdata_chain"));
}

/// XML 链解析：注释与空白混合。
#[test]
fn xml_comments_and_whitespace_ignored() {
    let bus = FlowBus::new();
    bus.register("a", cmp(|_| async { Ok(Value::Null) }));
    let xml = r#"
        <flow>
            <!-- 注释 -->
            <chain id="comment_chain">
                THEN(a)
            </chain>
        </flow>"#;
    let mut plan = RuleDefinitionPlan::new();
    let mut ids = HashSet::new();
    ParserHelper::parse_chain_document(&[xml.to_string()], &mut ids, &mut plan)
        .expect("注释链应解析");
    assert_eq!(ids.len(), 0, "跨文档清空后 ids 为空（Java 语义）");
}

/// XML 未闭合链报错。
#[test]
fn xml_unclosed_chain_rejected() {
    let xml = r#"<flow><chain id="open_chain">THEN(a)"#;
    let mut plan = RuleDefinitionPlan::new();
    let mut ids = HashSet::new();
    let error = ParserHelper::parse_chain_document(&[xml.to_string()], &mut ids, &mut plan)
        .expect_err("未闭合链应报错");
    assert!(error.to_string().contains("unclosed") || error.to_string().contains("parse error"));
}
