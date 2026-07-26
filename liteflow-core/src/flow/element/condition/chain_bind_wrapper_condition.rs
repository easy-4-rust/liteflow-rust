//! 对应 ChainBindWrapperCondition：把子链包装成可执行元素（子流程嵌套）。
//! EL 中直接写子链 id 即可引用（对齐 Java 的子流程语义）。

use super::Condition;
use crate::enums::ConditionTypeEnum;
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
    /// Chain 全局唯一，TAG 只保存在当前引用包装上。
    tag: Option<String>,
}

impl ChainBindWrapperCondition {
    pub fn new(wrapped_chain: Arc<Chain>) -> Self {
        let display = format!("chain_bind_wrapper_{}", wrapped_chain.id);
        Self {
            wrapped_chain,
            display,
            bind_data: Vec::new(),
            tag: None,
        }
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

    /// 设置当前子链引用标签，不修改全局 Chain。
    pub fn set_tag(&mut self, tag: impl Into<String>) {
        self.tag = Some(tag.into());
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
    fn tag(&self) -> Option<&str> {
        self.tag.as_deref()
    }
}

impl Condition for ChainBindWrapperCondition {
    fn condition_type(&self) -> ConditionTypeEnum {
        ConditionTypeEnum::ChainBindWrapper
    }
}
