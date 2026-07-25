//! 对应 ChainBindWrapperCondition：把子链包装成可执行元素（子流程嵌套）。
//! EL 中直接写子链 id 即可引用（对齐 Java 的子流程语义）。

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
}

impl ChainBindWrapperCondition {
    pub fn new(wrapped_chain: Arc<Chain>) -> Self {
        let display = format!("chain_bind_wrapper_{}", wrapped_chain.id);
        Self { wrapped_chain, display }
    }
    pub fn wrapped_chain(&self) -> &Arc<Chain> {
        &self.wrapped_chain
    }
}

#[async_trait]
impl Executable for ChainBindWrapperCondition {
    async fn execute(&self, ctx: &Ctx, _frame: &Frame) -> LFResult<Value> {
        self.wrapped_chain.execute(ctx).await
    }
    fn id(&self) -> &str {
        &self.display
    }
}
