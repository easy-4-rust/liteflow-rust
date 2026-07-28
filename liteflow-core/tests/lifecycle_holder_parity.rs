//! `LifeCycleHolder` Java 公共入口与动态阶段分派测试。

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use liteflow_core::flow::element::chain::Chain;
use liteflow_core::flow::element::node::Node;
use liteflow_core::{
    LifeCycle, LifeCycleHolder, NodeRef, PostProcessChainBuildLifeCycle,
    PostProcessChainExecuteLifeCycle, PostProcessFlowExecuteLifeCycle,
    PostProcessNodeBuildLifeCycle, PostProcessScriptEngineInitLifeCycle, Slot, cmp,
};
use serde_json::Value;

struct ScriptHook(Arc<AtomicUsize>);
struct ChainBuildHook(Arc<AtomicUsize>);
struct NodeBuildHook(Arc<AtomicUsize>);
struct FlowExecuteHook(Arc<AtomicUsize>);
struct ChainExecuteHook(Arc<AtomicUsize>);

impl LifeCycle for ScriptHook {
    fn register_life_cycle(self: Arc<Self>, life_cycle_holder: &mut LifeCycleHolder) {
        life_cycle_holder.script_engine_init.push(self);
    }
}

impl PostProcessScriptEngineInitLifeCycle for ScriptHook {
    fn post_process_after_script_engine_init(&self, _language: &str) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}

impl LifeCycle for ChainBuildHook {
    fn register_life_cycle(self: Arc<Self>, life_cycle_holder: &mut LifeCycleHolder) {
        life_cycle_holder.chain_build.push(self);
    }
}

impl PostProcessChainBuildLifeCycle for ChainBuildHook {
    fn post_process_before_chain_build(&self, _chain: &mut Chain) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }

    fn post_process_after_chain_build(&self, _chain: &Chain) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}

impl LifeCycle for NodeBuildHook {
    fn register_life_cycle(self: Arc<Self>, life_cycle_holder: &mut LifeCycleHolder) {
        life_cycle_holder.node_build.push(self);
    }
}

impl PostProcessNodeBuildLifeCycle for NodeBuildHook {
    fn post_process_before_node_build(&self, node: &mut Node) {
        node.set_tag("lifecycle");
        self.0.fetch_add(1, Ordering::SeqCst);
    }

    fn post_process_after_node_build(&self, node: &Node) {
        assert_eq!(node.get_tag(), Some("lifecycle"));
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}

impl LifeCycle for FlowExecuteHook {
    fn register_life_cycle(self: Arc<Self>, life_cycle_holder: &mut LifeCycleHolder) {
        life_cycle_holder.flow_execute.push(self);
    }
}

#[async_trait::async_trait]
impl PostProcessFlowExecuteLifeCycle for FlowExecuteHook {
    async fn post_process_before_flow_execute(&self, chain_id: &str, slot: &Slot) {
        assert_eq!(slot.get_chain_id(), chain_id);
        self.0.fetch_add(1, Ordering::SeqCst);
    }

    async fn post_process_after_flow_execute(&self, chain_id: &str, slot: &Slot) {
        assert_eq!(slot.get_chain_id(), chain_id);
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}

impl LifeCycle for ChainExecuteHook {
    fn register_life_cycle(self: Arc<Self>, life_cycle_holder: &mut LifeCycleHolder) {
        life_cycle_holder.chain_execute.push(self);
    }
}

#[async_trait::async_trait]
impl PostProcessChainExecuteLifeCycle for ChainExecuteHook {
    async fn post_process_before_chain_execute(&self, chain_id: &str, slot: &Slot) {
        assert_eq!(slot.get_chain_id(), chain_id);
        self.0.fetch_add(1, Ordering::SeqCst);
    }

    async fn post_process_after_chain_execute(&self, chain_id: &str, slot: &Slot) {
        assert_eq!(slot.get_chain_id(), chain_id);
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}

#[tokio::test]
async fn lifecycle_holder_classifies_invokes_and_cleans_all_java_stages() {
    let counter = Arc::new(AtomicUsize::new(0));
    let mut holder = LifeCycleHolder::default();

    // 对应 Java addLifeCycle 的五个 isAssignableFrom 分支；Rust 通过父 trait
    // 动态分派到同一组强类型列表。
    holder.add_life_cycle(Arc::new(ScriptHook(Arc::clone(&counter))));
    holder.add_life_cycle(Arc::new(ChainBuildHook(Arc::clone(&counter))));
    holder.add_life_cycle(Arc::new(NodeBuildHook(Arc::clone(&counter))));
    holder.add_life_cycle(Arc::new(FlowExecuteHook(Arc::clone(&counter))));
    holder.add_life_cycle(Arc::new(ChainExecuteHook(Arc::clone(&counter))));

    assert_eq!(
        holder
            .get_post_process_script_engine_init_life_cycle_list()
            .len(),
        1
    );
    assert_eq!(
        holder.get_post_process_chain_build_life_cycle_list().len(),
        1
    );
    assert_eq!(
        holder.get_post_process_node_build_life_cycle_list().len(),
        1
    );
    assert_eq!(
        holder.get_post_process_flow_execute_life_cycle_list().len(),
        1
    );
    assert_eq!(
        holder
            .get_post_process_chain_execute_life_cycle_list()
            .len(),
        1
    );

    holder.get_post_process_script_engine_init_life_cycle_list()[0]
        .post_process_after_script_engine_init("rhai");
    let mut chain = Chain::new("chain", Vec::new());
    holder.get_post_process_chain_build_life_cycle_list()[0]
        .post_process_before_chain_build(&mut chain);
    holder.get_post_process_chain_build_life_cycle_list()[0].post_process_after_chain_build(&chain);
    let mut node = Node::new(
        NodeRef::new("node"),
        Arc::new(cmp(|_| async { Ok(Value::Null) })),
    );
    holder.get_post_process_node_build_life_cycle_list()[0]
        .post_process_before_node_build(&mut node);
    holder.get_post_process_node_build_life_cycle_list()[0].post_process_after_node_build(&node);
    let slot = Slot::new("request".to_string(), "chain", Value::Null);
    holder.get_post_process_flow_execute_life_cycle_list()[0]
        .post_process_before_flow_execute("chain", &slot)
        .await;
    holder.get_post_process_flow_execute_life_cycle_list()[0]
        .post_process_after_flow_execute("chain", &slot)
        .await;
    holder.get_post_process_chain_execute_life_cycle_list()[0]
        .post_process_before_chain_execute("chain", &slot)
        .await;
    holder.get_post_process_chain_execute_life_cycle_list()[0]
        .post_process_after_chain_execute("chain", &slot)
        .await;
    assert_eq!(counter.load(Ordering::SeqCst), 9);

    holder.clean();
    assert!(
        holder
            .get_post_process_script_engine_init_life_cycle_list()
            .is_empty()
    );
    assert!(
        holder
            .get_post_process_chain_build_life_cycle_list()
            .is_empty()
    );
    assert!(
        holder
            .get_post_process_node_build_life_cycle_list()
            .is_empty()
    );
    assert!(
        holder
            .get_post_process_flow_execute_life_cycle_list()
            .is_empty()
    );
    assert!(
        holder
            .get_post_process_chain_execute_life_cycle_list()
            .is_empty()
    );
}
