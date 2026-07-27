//! Java `NodeComponent` 上下文访问与动态控制语义测试。

use std::sync::Arc;

use async_trait::async_trait;
use liteflow_core::core::NodeComponent;
use liteflow_core::el::NodeRef;
use liteflow_core::exception::{LFResult, LiteflowError};
use liteflow_core::flow::element::node::Node;
use liteflow_core::slot::{CmpContext, Ctx, DataBus, Frame, Slot};
use serde_json::{Value, json};

/// 用于验证 `NodeComponent` Java 命名入口的普通组件。
///
/// 对应 Java 测试中的自定义 `NodeComponent` 实现。
struct ContextAwareComponent;

#[async_trait]
impl NodeComponent for ContextAwareComponent {
    async fn process(&self, _ctx: &CmpContext) -> LFResult<Value> {
        Ok(json!({"processed": true}))
    }

    fn node_id(&self) -> &str {
        "context-node"
    }

    fn name(&self) -> &str {
        "上下文节点"
    }
}

/// 在 `process` 内动态选择异常继续或结束链路的组件。
///
/// 对应 Java `NodeComponent#setIsContinueOnError/setIsEnd` 的运行期用法。
struct DynamicControlComponent {
    continue_on_error: bool,
    end_chain: bool,
}

#[async_trait]
impl NodeComponent for DynamicControlComponent {
    async fn process(&self, ctx: &CmpContext) -> LFResult<Value> {
        self.set_step_data(ctx, json!({"control": "visited"}));
        if self.end_chain {
            self.set_is_end(ctx, true);
        }
        if self.continue_on_error {
            self.set_is_continue_on_error(ctx, true);
            return Err(LiteflowError::Custom("dynamic failure".to_string()));
        }
        Ok(json!("done"))
    }

    fn node_id(&self) -> &str {
        "dynamic-node"
    }
}

fn context(slot: Arc<Slot>, node: NodeRef, frame: Frame) -> CmpContext {
    CmpContext {
        inner: slot,
        node,
        frame,
    }
}

#[test]
fn java_named_context_helpers_read_and_mutate_real_slot_and_frame_state() {
    let slot = Arc::new(Slot::new(
        "request-node-component".to_string(),
        "main-chain",
        json!({"root": true}),
    ));
    slot.set_chain_req_data("main-chain", json!({"order_id": 7}));
    slot.insert_context_bean(
        "order",
        Arc::new(json!({"customer": {"name": "Ada"}, "status": "new"})),
    );

    let mut node_ref = NodeRef::new("context-node");
    node_ref.tag = Some("blue".to_string());
    node_ref.data = Some(r#"[{"sku":"A"},{"sku":"B"}]"#.to_string());
    node_ref
        .bind
        .push(("items".to_string(), "[1,2,3]".to_string()));

    let frame = Frame::root()
        .push(3, Some(json!("outer")))
        .push(7, Some(json!("inner")))
        .with_current_chain_id("child-chain")
        .with_runtime_id(42);
    let ctx = context(Arc::clone(&slot), node_ref, frame);
    let component = ContextAwareComponent;

    let slot_index = DataBus::offer_slot(Arc::clone(&slot));
    assert_eq!(component.get_slot_index(&ctx), Some(slot_index));
    assert!(Arc::ptr_eq(&component.get_slot(&ctx), &slot));
    assert_eq!(component.get_node_id(), "context-node");
    assert_eq!(component.get_name(), "上下文节点");
    assert_eq!(component.get_display_name(), "context-node(上下文节点)");
    assert_eq!(component.get_tag(&ctx), Some("blue"));
    assert_eq!(component.get_chain_id(&ctx), "main-chain");
    assert_eq!(component.get_curr_chain_id(&ctx), "child-chain");
    assert_eq!(component.get_curr_chain_runtime_id(&ctx), Some(42));
    assert_eq!(
        component.get_request_data(&ctx),
        Some(json!({"order_id": 7}))
    );

    let first_bean = component
        .get_first_context_bean(&ctx)
        .expect("应返回首个上下文 Bean")
        .downcast::<Value>()
        .expect("上下文 Bean 应保持 JSON 类型");
    assert_eq!(first_bean["status"], "new");
    assert!(component.get_context_bean(&ctx, "order").is_some());
    assert_eq!(
        component.get_context_value(&ctx, "order.customer.name"),
        Some(json!("Ada"))
    );
    assert!(component.set_context_value(&ctx, "order.setStatus", &[json!("paid")]));
    assert_eq!(
        component.get_context_value(&ctx, "order.status"),
        Some(json!("paid"))
    );

    assert_eq!(
        component.get_cmp_data_list(&ctx),
        Some(vec![json!({"sku": "A"}), json!({"sku": "B"})])
    );
    assert_eq!(
        component.get_bind_data(&ctx, "items").as_deref(),
        Some("[1,2,3]")
    );
    assert_eq!(
        component.get_bind_data_list(&ctx, "items"),
        Some(vec![json!(1), json!(2), json!(3)])
    );
    assert_eq!(component.get_loop_index(&ctx), Some(7));
    assert_eq!(component.get_pre_loop_index(&ctx), Some(3));
    assert_eq!(component.get_pre_n_loop_index(&ctx, 1), Some(3));
    assert_eq!(component.get_curr_loop_obj(&ctx), Some(json!("inner")));
    assert_eq!(component.get_pre_loop_obj(&ctx), Some(json!("outer")));

    component.send_private_delivery_data(&ctx, "context-node", json!("message"));
    assert_eq!(
        component.get_private_delivery_data(&ctx),
        Some(json!("message"))
    );
    assert_eq!(component.get_private_delivery_data(&ctx), None);
    assert!(DataBus::release_slot(slot_index));
}

#[tokio::test]
async fn java_named_dynamic_controls_enter_the_real_node_execution_path() {
    let continue_slot = Arc::new(Slot::new(
        "request-continue".to_string(),
        "main",
        Value::Null,
    ));
    let continue_ctx = Ctx::new(Arc::clone(&continue_slot));
    let continue_frame = Frame::root();
    let continue_node = Node::new(
        NodeRef::new("dynamic-node"),
        Arc::new(DynamicControlComponent {
            continue_on_error: true,
            end_chain: false,
        }),
    );

    assert_eq!(
        continue_node
            .execute(&continue_ctx, &continue_frame)
            .await
            .expect("动态 continue-on-error 应吞掉组件错误"),
        Value::Null
    );
    let continue_steps = continue_slot.get_execute_steps();
    assert_eq!(continue_steps.len(), 1);
    assert_eq!(
        continue_steps[0].get_step_data(),
        Some(&json!({"control": "visited"}))
    );

    let end_slot = Arc::new(Slot::new("request-end".to_string(), "main", Value::Null));
    let end_ctx = Ctx::new(end_slot);
    let end_frame = Frame::root();
    let end_node = Node::new(
        NodeRef::new("dynamic-node"),
        Arc::new(DynamicControlComponent {
            continue_on_error: false,
            end_chain: true,
        }),
    );

    assert!(matches!(
        end_node.execute(&end_ctx, &end_frame).await,
        Err(LiteflowError::ChainEnd)
    ));
}

#[tokio::test]
async fn item_result_java_entry_reads_the_result_written_by_node_execution() {
    let slot = Arc::new(Slot::new("request-result".to_string(), "main", Value::Null));
    let ctx = Ctx::new(Arc::clone(&slot));
    let frame = Frame::root();
    let component = Arc::new(ContextAwareComponent);
    let node = Node::new(NodeRef::new("context-node"), component.clone());

    node.execute(&ctx, &frame)
        .await
        .expect("节点应执行成功并写入结果缓存");
    let cmp_context = context(slot, NodeRef::new("context-node"), frame);
    assert_eq!(
        component.get_item_result_meta_value(&cmp_context),
        Some(json!({"processed": true}))
    );
}
