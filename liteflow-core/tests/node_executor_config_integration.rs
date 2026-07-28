//! 全局 nodeExecutorClass 到真实 Rust 节点执行器的执行链集成测试。

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use liteflow_core::NodeComponent;
use liteflow_core::el::NodeRef;
use liteflow_core::exception::LiteflowError;
use liteflow_core::flow::element::node::Node;
use liteflow_core::flow::executor::{NodeExecutor, NodeExecutorHelper};
use liteflow_core::property::{LiteflowConfig, LiteflowConfigGetter};
use liteflow_core::slot::{CmpContext, Ctx, Frame, Slot};
use serde_json::Value;

const CUSTOM_EXECUTOR_CLASS: &str = "example.CountingNodeExecutor";

/// 记录是否真正进入自定义执行器的测试实现。
struct CountingNodeExecutor {
    calls: Arc<AtomicUsize>,
}

#[async_trait::async_trait]
impl NodeExecutor for CountingNodeExecutor {
    async fn execute(&self, node: &Node, ctx: &Ctx, frame: &Frame) -> Result<Value, LiteflowError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        node.execute_once(ctx, frame).await
    }
}

/// 记录节点业务逻辑是否执行的测试组件。
struct CountingComponent {
    calls: Arc<AtomicUsize>,
}

#[async_trait::async_trait]
impl NodeComponent for CountingComponent {
    async fn process(&self, _ctx: &CmpContext) -> Result<Value, LiteflowError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(Value::String("executed".to_string()))
    }
}

fn node_context() -> (Ctx, Frame) {
    let slot = Arc::new(Slot::new(
        "node-executor-config".to_string(),
        "chain",
        Value::Null,
    ));
    (Ctx::new(slot), Frame::root())
}

/// 验证已注册类名进入真实节点执行，并且未知类名明确失败而非退回默认执行器。
#[tokio::test]
async fn configured_node_executor_is_resolved_and_unknown_class_fails() {
    let helper = NodeExecutorHelper::load_instance();
    let executor_calls = Arc::new(AtomicUsize::new(0));
    helper.register_named_node_executor(
        CUSTOM_EXECUTOR_CLASS,
        Arc::new(CountingNodeExecutor {
            calls: executor_calls.clone(),
        }),
    );

    let mut config = LiteflowConfig::default();
    config.set_node_executor_class(CUSTOM_EXECUTOR_CLASS);
    LiteflowConfigGetter::set_liteflow_config(config);

    let component_calls = Arc::new(AtomicUsize::new(0));
    let node = Node::new(
        NodeRef::new("configured_node"),
        Arc::new(CountingComponent {
            calls: component_calls.clone(),
        }),
    );
    let (ctx, frame) = node_context();
    let result = node.execute(&ctx, &frame).await;

    assert_eq!(result.unwrap(), Value::String("executed".to_string()));
    assert_eq!(executor_calls.load(Ordering::SeqCst), 1);
    assert_eq!(component_calls.load(Ordering::SeqCst), 1);

    let unknown_class = "missing.UnknownNodeExecutor";
    let mut unknown_config = LiteflowConfig::default();
    unknown_config.set_node_executor_class(unknown_class);
    LiteflowConfigGetter::set_liteflow_config(unknown_config);

    let error = node.execute(&ctx, &frame).await.unwrap_err();
    assert!(
        matches!(
            error,
            LiteflowError::NodeClassNotFound(message)
                if message.contains(unknown_class)
        ),
        "未知执行器类名必须返回 NodeClassNotFound"
    );
    assert_eq!(
        component_calls.load(Ordering::SeqCst),
        1,
        "解析失败时不得执行组件业务逻辑"
    );

    helper.remove_named_node_executor(CUSTOM_EXECUTOR_CLASS);
    LiteflowConfigGetter::clean();
}
