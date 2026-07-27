//! `FallbackNode` Java 命名入口与直接执行语义测试。

use std::sync::Arc;

use dashmap::DashMap;
use liteflow_core::el::NodeRef;
use liteflow_core::flow::element::FallbackNode;
use liteflow_core::slot::{CmpContext, Frame, Slot};
use liteflow_core::{LFResult, NodeComponent, NodeTypeEnum};
use serde_json::{Value, json};

struct ProbeComponent;

#[async_trait::async_trait]
impl NodeComponent for ProbeComponent {
    async fn process(&self, context: &CmpContext) -> LFResult<Value> {
        context.set_data("fallback_direct", json!(true));
        Ok(json!("resolved"))
    }

    fn node_id(&self) -> &str {
        "actualNode"
    }

    fn node_type(&self) -> Option<NodeTypeEnum> {
        Some(NodeTypeEnum::Common)
    }
}

#[tokio::test]
async fn fallback_node_java_entries_resolve_and_execute_a_real_node() {
    let nodes: Arc<DashMap<String, Arc<dyn NodeComponent>>> = Arc::new(DashMap::new());
    let fallback_nodes: Arc<DashMap<String, Arc<dyn NodeComponent>>> = Arc::new(DashMap::new());
    nodes.insert("expectedNode".to_string(), Arc::new(ProbeComponent));

    let mut fallback = FallbackNode::new(
        "expectedNode",
        NodeTypeEnum::Common,
        Arc::clone(&nodes),
        Arc::clone(&fallback_nodes),
    );
    assert_eq!(fallback.get_expected_node_id(), "expectedNode");
    assert_eq!(fallback.get_type(), NodeTypeEnum::Fallback);
    assert!(std::ptr::eq(fallback.clone(), &fallback));
    assert_eq!(fallback.get_id().as_deref(), Some("actualNode"));
    assert!(fallback.get_instance().is_some());

    let slot = Arc::new(Slot::new(
        "fallback-direct".to_string(),
        "fallback-chain",
        Value::Null,
    ));
    let context = CmpContext {
        inner: Arc::clone(&slot),
        node: NodeRef::new("expectedNode"),
        frame: Frame::root(),
    };
    assert!(fallback.is_access(&context).expect("原节点应可访问"));
    assert_eq!(
        fallback
            .execute(&context)
            .await
            .expect("直接入口应走真实 Node 执行链"),
        json!("resolved")
    );
    assert_eq!(context.get_data("fallback_direct"), Some(json!(true)));
    assert_eq!(
        fallback.get_item_result_meta_value(&context),
        Some(json!("resolved"))
    );

    fallback.set_expected_node_id("missingNode");
    assert_eq!(fallback.get_expected_node_id(), "missingNode");
    assert!(fallback.get_instance().is_none());
    assert!(fallback.get_id().is_none());
}
