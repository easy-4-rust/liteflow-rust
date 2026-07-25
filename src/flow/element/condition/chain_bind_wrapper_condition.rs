//! 对应 flow.element.condition.ChainBindWrapperCondition：子链包装。
//!
//! Java 语义：一个 Chain 被其他链引用时，用 ChainBindWrapperCondition 包裹后执行；
//! bind 数据挂在 wrapper 上（Condition.bindData），不会污染共享的子链对象。

use crate::exception::LFResult;
use crate::flow::element::chain::Chain;
use crate::flow::element::executable::Executable;
use crate::slot::{Ctx, Frame};
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;

pub struct ChainBindWrapperCondition {
    wrapped_chain: Arc<Chain>,
    display: String,
    /// 2.16：Chain bind 场景下 bind 数据存在 wrapper 上（putBindData），
    /// 避免多个 chain 引用同一子链时的 bind 数据污染
    bind_data: Vec<(String, String)>,
}

impl ChainBindWrapperCondition {
    pub fn new(wrapped_chain: Arc<Chain>) -> Self {
        let display = format!("chain_bind_wrapper_{}", wrapped_chain.id);
        Self { wrapped_chain, display, bind_data: Vec::new() }
    }
    /// putBindData（对应 Java Condition.putBindData）
    pub fn put_bind_data(&mut self, key: impl Into<String>, value: impl Into<String>) {
        let k = key.into();
        self.bind_data.retain(|(ek, _)| *ek != k);
        self.bind_data.push((k, value.into()));
    }
    pub fn wrapped_chain(&self) -> &Arc<Chain> {
        &self.wrapped_chain
    }
}

#[async_trait]
impl Executable for ChainBindWrapperCondition {
    async fn execute(&self, ctx: &Ctx, frame: &Frame) -> LFResult<Value> {
        // bind 数据随执行路径下传（对应 conditionStack 上的 bindData 查找）
        let frame = frame.push_bind(&self.bind_data);
        self.wrapped_chain.execute_with_frame(ctx, &frame).await
    }
    fn id(&self) -> &str {
        &self.display
    }
}
