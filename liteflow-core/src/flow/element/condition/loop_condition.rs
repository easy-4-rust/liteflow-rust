// 循环次数

use std::any::Any;
use std::sync::Arc;

use crate::context::Context;
use crate::flow::element::{Condition, ConditionKey, Executable};
use crate::r#loop::LoopOperation;
use crate::slot::DataBus;

/// LOOP 条件
///
/// 循环次数判断组件，条件语法如下：
///
/// - `LOOP(5).DO(THEN(a, b));`
/// - `LOOP(5).PARALLEL_DO(THEN(a, b));`
///
/// 其中，`5` 表示循环次数，从 0 到 4，
///
/// 以上代码均可用本类来表示
#[derive(Clone)]
pub struct LoopCondition {
    /// 循环次数
    pub r#loop: Box<dyn Executable>,
    /// 循环体
    pub r#do: Option<Box<dyn Executable>>,
    /// 循环操作
    pub loop_operation: LoopOperation,
}

impl LoopCondition {
    /// 获取循环体
    ///
    /// 如果循环体未指定，则抛出异常
    fn get_do(&self) -> &dyn Executable {
        // 在处理循环体时，如果循环体未指定，则抛出异常
        self.r#do
            .as_deref()
            .expect("LoopCondition must specify a do executable")
    }
}

#[async_trait::async_trait]
impl Condition for LoopCondition {
    /// 执行循环条件
    ///
    /// 根据 `loop_operation` 的值，选择串行循环或并行循环
    ///
    /// 如果是串行循环，则重复执行循环体，直到 `loop_count` 返回 `false`
    ///
    /// 如果是并行循环，则根据 `loop_operation` 的值，决定并行循环的方式
    /// 如果是 `FOR` 操作，则重复执行循环体，直到 `loop_count` 返回 `false`
    async fn execute(&self, slot_key: usize) -> anyhow::Result<()> {
        match self.loop_operation {
            // 串行循环
            LoopOperation::LOOP => {
                while self.loop_count(slot_key).await? {
                    self.get_do().execute(slot_key).await?;
                }
                Ok(())
            }
            // 并行循环
            LoopOperation::FOR => {
                // 并行循环，根据 `loop_operation` 的值，决定并行循环的方式
                // 如果是 `FOR` 操作，则重复执行循环体，直到 `loop_count` 返回 `false`
                while self.loop_count(slot_key).await? {
                    self.get_do().execute(slot_key).await?;
                }
                Ok(())
            }
        }
    }

    /// 循环条件
    ///
    /// 循环条件只需要循环体，不需要其他子组件
    async fn loop_count(&self, slot_key: usize) -> anyhow::Result<bool> {
        let slot = DataBus::get_slot(slot_key).expect("slot not found");
        let context_idx = self.get_context_bean_idx(slot_key);
        let context: Arc<dyn Context> = slot.get_context(context_idx)?;

        self.r#loop
            .execute_process(slot_key, |cmp| cmp.process_loop_count(context))
            .await
    }
}

impl std::fmt::Debug for LoopCondition {
    /// 打印循环条件
    ///
    /// 打印循环次数、循环体和循环操作
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LoopCondition")
            .field("loop", &self.r#loop.id())
            .field("do", &self.get_do().id())
            .field("loop_operation", &self.loop_operation)
            .finish()
    }
}
