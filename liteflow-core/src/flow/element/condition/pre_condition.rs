//! 对应 Java 类：com.yomahub.liteflow.flow.element.condition.PreCondition
//!
//! 前置 Condition，按列表顺序执行全部可执行项。

use super::{Condition, ConditionBase};
use crate::enums::ConditionTypeEnum;
use crate::exception::LFResult;
use crate::flow::element::executable::Executable;
use crate::slot::{Ctx, Frame};
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Clone)]
pub struct PreCondition {
    base: ConditionBase,
    executable_list: Vec<Arc<dyn Executable>>,
}

impl PreCondition {
    /// 创建前置 Condition。
    ///
    /// 参数 `item` 是 PRE 中需要执行的真实表达式；返回值保存同一共享执行对象。
    /// 对应 Java: `PreCondition#PreCondition` 的 Builder 装配结果。
    #[must_use]
    pub fn new(item: Arc<dyn Executable>) -> Self {
        Self {
            base: ConditionBase::default(),
            executable_list: vec![item],
        }
    }

    /// 执行 PRE 条件主体。对应 Java: `PreCondition#executeCondition`。
    pub async fn execute_condition(&self, ctx: &Ctx, frame: &Frame) -> LFResult<Value> {
        <Self as Executable>::execute(self, ctx, frame).await
    }

    /// 返回条件类型。对应 Java: `PreCondition#getConditionType`。
    #[must_use]
    pub fn get_condition_type(&self) -> ConditionTypeEnum {
        ConditionTypeEnum::Pre
    }

    /// 返回条件类型的 Rust 惯用别名。
    pub fn condition_type(&self) -> ConditionTypeEnum {
        self.get_condition_type()
    }
}

#[async_trait]
impl Executable for PreCondition {
    async fn execute(&self, ctx: &Ctx, frame: &Frame) -> LFResult<Value> {
        super::execute_condition_with_lifecycle(self, ctx, frame, async {
            // 对应 Java PreCondition#executeCondition：按 executableList 插入顺序
            // 串行执行，首个异常立即停止并向上传播。
            for executable in &self.executable_list {
                executable.execute(ctx, frame).await?;
            }
            Ok(Value::Null)
        })
        .await
    }
    fn collect_node_ids(&self) -> Vec<String> {
        Condition::get_all_node_in_condition(self)
    }
    fn id(&self) -> &str {
        "PRE"
    }
    fn is_pre_or_finally(&self) -> bool {
        true
    }
}

impl Condition for PreCondition {
    fn condition_base(&self) -> &ConditionBase {
        &self.base
    }

    fn condition_base_mut(&mut self) -> &mut ConditionBase {
        &mut self.base
    }

    fn typed_executable_group(&self) -> HashMap<String, Vec<Arc<dyn Executable>>> {
        HashMap::from([("DEFAULT_KEY".to_string(), self.executable_list.clone())])
    }

    fn replace_typed_executable_group(
        &mut self,
        group_key: &str,
        executable_list: Vec<Arc<dyn Executable>>,
    ) -> bool {
        if group_key == "DEFAULT_KEY" {
            self.executable_list = executable_list;
            true
        } else {
            false
        }
    }

    fn condition_type(&self) -> ConditionTypeEnum {
        PreCondition::condition_type(self)
    }
}
