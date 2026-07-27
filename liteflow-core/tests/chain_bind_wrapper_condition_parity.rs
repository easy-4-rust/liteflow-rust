//! Java `ChainBindWrapperCondition` 子链包装与执行语义回归测试。

use std::sync::Arc;

use liteflow_core::enums::ConditionTypeEnum;
use liteflow_core::flow::element::chain::Chain;
use liteflow_core::flow::element::condition::chain_bind_wrapper_condition::ChainBindWrapperCondition;
use liteflow_core::{Ctx, Frame, Slot};
use serde_json::{Value, json};

#[tokio::test]
async fn java_named_api_executes_the_same_wrapped_chain_without_mutating_it() {
    let chain = Arc::new(Chain::new("sub_chain", Vec::new()));
    let wrapper = ChainBindWrapperCondition::new(Arc::clone(&chain));

    assert_eq!(
        wrapper.get_condition_type(),
        ConditionTypeEnum::ChainBindWrapper
    );
    assert_eq!(wrapper.get_id(), "chain_bind_wrapper_sub_chain");
    assert!(Arc::ptr_eq(wrapper.get_wrapped_chain(), &chain));

    let ctx = Ctx::new(Arc::new(Slot::new(
        "request-1".to_string(),
        "main_chain",
        json!({}),
    )));
    assert_eq!(
        wrapper
            .execute_condition(&ctx, &Frame::default())
            .await
            .expect("空子链应正常执行"),
        Value::Null
    );
    assert_eq!(chain.get_chain_id(), "sub_chain");
}
