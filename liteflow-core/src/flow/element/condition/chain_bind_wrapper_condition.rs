//! 对应 Java: `com.yomahub.liteflow.flow.element.condition.ChainBindWrapperCondition`。

use super::{Condition, ConditionBase};
use crate::enums::ConditionTypeEnum;
use crate::exception::LFResult;
use crate::flow::element::chain::Chain;
use crate::flow::element::executable::Executable;
use crate::slot::{Ctx, Frame};
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;

/// Chain bind 包装条件。
///
/// 对 Chain 执行 bind 时使用独立包装对象持有 bind 数据，不直接修改被多个父链共享
/// 的子 Chain，从而避免引用同一个子链时发生数据污染。
///
/// 对应 Java: `com.yomahub.liteflow.flow.element.condition.ChainBindWrapperCondition`。
#[derive(Clone)]
pub struct ChainBindWrapperCondition {
    base: ConditionBase,
    wrapped_chain: Arc<Chain>,
    display: String,
}

impl ChainBindWrapperCondition {
    /// 创建子链 bind 包装条件。
    ///
    /// # 参数
    /// - `wrapped_chain`: 被包装且可由多个父链共享的子 Chain。
    ///
    /// # 返回
    /// 持有独立 Condition 状态的包装对象。
    ///
    /// 对应 Java: `ChainBindWrapperCondition#ChainBindWrapperCondition`。
    pub fn new(wrapped_chain: Arc<Chain>) -> Self {
        let display = format!("chain_bind_wrapper_{}", wrapped_chain.id);
        Self {
            base: ConditionBase::default(),
            wrapped_chain,
            display,
        }
    }
    /// putBindData（对应 Java Condition.putBindData）
    pub fn put_bind_data(&mut self, key: impl Into<String>, value: impl Into<String>) {
        <Self as Condition>::put_bind_data(self, key, value);
    }
    pub fn wrapped_chain(&self) -> &Arc<Chain> {
        &self.wrapped_chain
    }

    /// 返回被包装的子 Chain。
    ///
    /// # 返回
    /// 与实际执行路径共享的同一个 `Arc<Chain>`。
    ///
    /// 对应 Java: `ChainBindWrapperCondition#getWrappedChain`。
    #[must_use]
    pub fn get_wrapped_chain(&self) -> &Arc<Chain> {
        self.wrapped_chain()
    }

    /// 执行被包装的子 Chain。
    ///
    /// # 参数
    /// - `ctx`: 当前 Slot 的执行上下文。
    /// - `frame`: 当前父链执行帧。
    ///
    /// # 返回
    /// 返回子 Chain 的执行结果或原始执行错误。
    ///
    /// bind 数据只压入本次执行帧；`Chain#execute_with_frame` 会写入子链当前 ID，
    /// 因而不会修改全局共享的 Chain 元数据。对应 Java:
    /// `ChainBindWrapperCondition#executeCondition`。
    pub async fn execute_condition(&self, ctx: &Ctx, frame: &Frame) -> LFResult<Value> {
        // 每次执行创建独立帧，把包装对象上的 bind 数据沿当前路径传给子链。
        let frame = frame.push_bind(self.base.bind_data());
        self.wrapped_chain.execute_with_frame(ctx, &frame).await
    }

    /// 返回条件类型。
    ///
    /// # 返回
    /// 固定返回 `ConditionTypeEnum::ChainBindWrapper`。
    ///
    /// 对应 Java: `ChainBindWrapperCondition#getConditionType`。
    #[must_use]
    pub fn get_condition_type(&self) -> ConditionTypeEnum {
        <Self as Condition>::condition_type(self)
    }

    /// 返回包装条件 ID。
    ///
    /// # 返回
    /// `chain_bind_wrapper_` 与被包装子链 ID 的拼接结果。
    ///
    /// 对应 Java: `ChainBindWrapperCondition#getId`。
    #[must_use]
    pub fn get_id(&self) -> &str {
        &self.display
    }

    /// 设置当前子链引用标签，不修改全局 Chain。
    pub fn set_tag(&mut self, tag: impl Into<String>) {
        <Self as Condition>::set_tag(self, tag);
    }
}

#[async_trait]
impl Executable for ChainBindWrapperCondition {
    async fn execute(&self, ctx: &Ctx, frame: &Frame) -> LFResult<Value> {
        super::execute_condition_with_lifecycle(
            self,
            ctx,
            frame,
            self.execute_condition(ctx, frame),
        )
        .await
    }
    fn collect_node_ids(&self) -> Vec<String> {
        self.wrapped_chain.collect_node_ids()
    }
    fn id(&self) -> &str {
        &self.display
    }
    fn tag(&self) -> Option<&str> {
        <Self as Condition>::get_tag(self)
    }
}

impl Condition for ChainBindWrapperCondition {
    fn condition_base(&self) -> &ConditionBase {
        &self.base
    }

    fn condition_base_mut(&mut self) -> &mut ConditionBase {
        &mut self.base
    }

    fn condition_type(&self) -> ConditionTypeEnum {
        ConditionTypeEnum::ChainBindWrapper
    }
}
