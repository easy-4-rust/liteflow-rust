//! 生命周期接口在真实构建与嵌套执行调用链中的 Java 语义回归。
//!
//! 对应 Java:
//! - `com.yomahub.liteflow.lifecycle.PostProcessNodeBuildLifeCycle`
//! - `com.yomahub.liteflow.lifecycle.PostProcessChainExecuteLifeCycle`
//! - `com.yomahub.liteflow.flow.element.Chain#execute`

use async_trait::async_trait;
use liteflow_core::flow::element::node::Node;
use liteflow_core::{
    CmpContext, FlowBus, LifeCycle, LifeCycleHolder, LiteflowError, NodeComponent,
    PostProcessChainExecuteLifeCycle, PostProcessNodeBuildLifeCycle, Slot,
};
use serde_json::Value;
use std::sync::{Arc, Mutex};

struct NodeBuildRecorder {
    events: Arc<Mutex<Vec<String>>>,
}

impl LifeCycle for NodeBuildRecorder {
    fn register_life_cycle(self: Arc<Self>, life_cycle_holder: &mut LifeCycleHolder) {
        life_cycle_holder.node_build.push(self);
    }
}

impl PostProcessNodeBuildLifeCycle for NodeBuildRecorder {
    fn post_process_before_node_build(&self, node: &mut Node) {
        self.events
            .lock()
            .unwrap()
            .push(format!("node_before:{}", node.get_id()));
        node.set_tag("from_before");
    }

    fn post_process_after_node_build(&self, node: &Node) {
        self.events.lock().unwrap().push(format!(
            "node_after:{}:{}",
            node.get_id(),
            node.get_tag().unwrap_or_default()
        ));
    }
}

struct ChainExecuteRecorder {
    events: Arc<Mutex<Vec<String>>>,
}

impl LifeCycle for ChainExecuteRecorder {
    fn register_life_cycle(self: Arc<Self>, life_cycle_holder: &mut LifeCycleHolder) {
        life_cycle_holder.chain_execute.push(self);
    }
}

#[async_trait]
impl PostProcessChainExecuteLifeCycle for ChainExecuteRecorder {
    async fn post_process_before_chain_execute(&self, chain_id: &str, slot: &Slot) {
        self.events.lock().unwrap().push(format!(
            "chain_before:{chain_id}:{}:{}",
            slot.get_chain_id(),
            !slot.get_request_id().is_empty()
        ));
    }

    async fn post_process_after_chain_execute(&self, chain_id: &str, slot: &Slot) {
        self.events.lock().unwrap().push(format!(
            "chain_after:{chain_id}:{}:{}",
            slot.get_chain_id(),
            slot.get_exception().is_none()
        ));
    }
}

struct TagRecorder {
    tags: Arc<Mutex<Vec<String>>>,
}

#[async_trait]
impl NodeComponent for TagRecorder {
    async fn process(&self, context: &CmpContext) -> Result<Value, LiteflowError> {
        self.tags
            .lock()
            .unwrap()
            .push(context.node.tag.clone().unwrap_or_default());
        Ok(Value::Null)
    }
}

#[tokio::test]
async fn node_mutation_and_nested_chain_slot_reach_real_runtime() {
    let bus = FlowBus::new();
    let build_events = Arc::new(Mutex::new(Vec::new()));
    let chain_events = Arc::new(Mutex::new(Vec::new()));
    let tags = Arc::new(Mutex::new(Vec::new()));

    bus.register_node_build_hook(Arc::new(NodeBuildRecorder {
        events: build_events.clone(),
    }));
    bus.register_chain_execute_hook(Arc::new(ChainExecuteRecorder {
        events: chain_events.clone(),
    }));
    bus.register("a", TagRecorder { tags: tags.clone() });
    bus.add_chain("child", "THEN(a)").unwrap();
    bus.add_chain("main", "THEN(child)").unwrap();

    let response = bus.execute("main").await;
    assert!(response.is_success());
    assert_eq!(*tags.lock().unwrap(), ["from_before"]);
    assert_eq!(
        *build_events.lock().unwrap(),
        ["node_before:a", "node_after:a:from_before"]
    );
    assert_eq!(
        *chain_events.lock().unwrap(),
        [
            "chain_before:main:main:true",
            "chain_before:child:main:true",
            "chain_after:child:main:true",
            "chain_after:main:main:true",
        ]
    );
}
