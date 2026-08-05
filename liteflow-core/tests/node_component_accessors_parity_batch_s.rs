//! NodeComponent 访问器全量调用补测（批次 S）。
//!
//! 在真实组件执行上下文中调用 Java 对等访问器：
//! - `NodeComponent#getCmpData/getCmpDataAs/getCmpDataList/getBindData/
//!   getBindDataAs/getBindDataList/getChainId/getContextBean/getLoopIndex/
//!   getRequestData` 等默认方法

use liteflow_core::slot::Slot;
use liteflow_core::{CmpContext, FlowBus, Frame, NodeComponent, NodeRef, cmp};
use serde_json::{Value, json};
use std::sync::Arc;

/// 在组件内调用全部 Java 对等访问器。
#[tokio::test]
async fn node_component_accessors_in_real_context() {
    let bus = FlowBus::new();
    bus.register(
        "accessor",
        cmp(|ctx| async move {
            // getChainId（Java NodeComponent#getChainId）
            let chain_id = ctx.chain_id();
            assert_eq!(chain_id, "accessor_chain");

            // cmpData：通过节点组件数据键读写
            ctx.set_data("cmp_key", json!({"k": "v"}));
            assert_eq!(ctx.get_data("cmp_key"), Some(json!({"k": "v"})));

            // bindData：节点级绑定数据读取（未设置时 None）
            assert!(ctx.bind_data("tenant").is_none());

            // loop 栈
            assert_eq!(ctx.loop_index(), None);

            // 请求数据与上下文 Bean
            assert!(ctx.request_data::<Value>().is_some());
            assert!(ctx.bean::<u32>("ctx_bean").is_some());

            // slot 快照
            let _slot_index = ctx.slot_index();

            ctx.set_data("accessor", json!("accessor-done"));
            Ok(Value::Null)
        }),
    );
    bus.add_chain("accessor_chain", "THEN(accessor)").unwrap();
    let response = bus
        .execute_with(
            "accessor_chain",
            json!({"input": 1}),
            vec![("ctx_bean".to_string(), Arc::new(7_u32))],
        )
        .await;
    assert!(response.is_success(), "{}", response.message);
    assert_eq!(response.data("accessor"), Some(json!("accessor-done")));
}

/// 通过 NodeComponent 默认方法访问器读取节点元数据。
#[tokio::test]
async fn node_component_default_accessors_read_metadata() {
    let slot = Arc::new(Slot::new("RID-ACC".to_string(), "meta_chain", Value::Null));
    let frame = Frame::root().push(0, Some(json!({"item": 1})));
    let context = CmpContext {
        inner: slot,
        node: NodeRef::new("meta_node"),
        frame,
    };

    let component: Arc<dyn NodeComponent> = Arc::new(cmp(|_| async { Ok(Value::Null) }));
    // 默认方法：链 ID / 循环索引 / 上下文 Bean
    assert_eq!(component.get_chain_id(&context), "meta_chain");
    assert_eq!(component.get_loop_index(&context), Some(0));
    assert!(component.get_context_bean(&context, "missing").is_none());
    // cmpData / bindData 默认返回 None（未设置时 Java 语义）
    assert!(component.get_cmp_data(&context).is_none());
    assert!(component.get_bind_data(&context, "key").is_none());
    // 列表访问器
    assert!(component.get_cmp_data_list(&context).is_none());
    assert!(component.get_bind_data_list(&context, "key").is_none());
}

/// 组件元数据访问器的类型化转换错误路径。
#[tokio::test]
async fn node_component_typed_conversion_errors() {
    let bus = FlowBus::new();
    bus.register(
        "typed",
        cmp(|ctx| async move {
            ctx.set_data("typed_key", json!({"n": 1}));
            // 字符串目标类型的转换失败返回 None
            let result = ctx.get_data_as::<String>("typed_key");
            assert!(result.is_none());
            Ok(Value::Null)
        }),
    );
    bus.add_chain("typed_chain", "THEN(typed)").unwrap();
    let response = bus.execute("typed_chain").await;
    assert!(response.is_success());
}
