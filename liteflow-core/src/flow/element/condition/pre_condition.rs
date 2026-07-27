//! 对应 Java 类：com.yomahub.liteflow.flow.element.condition.PreCondition
//!
//! 前置 Condition。
//!
//! 差异说明：
//! - Java PreCondition 持有 executableList 并循环执行多个可执行项；Rust 端
//!   EL 中 PRE(...) 只包裹单个表达式，由 builder 保证单 item，故持单字段。

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
    item: Arc<dyn Executable>,
}

impl PreCondition {
    pub fn new(item: Arc<dyn Executable>) -> Self {
        Self {
            base: ConditionBase::default(),
            item,
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
            self.item.execute(ctx, frame).await
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
        HashMap::from([("DEFAULT_KEY".to_string(), vec![Arc::clone(&self.item)])])
    }

    fn replace_typed_executable_group(
        &mut self,
        group_key: &str,
        executable_list: Vec<Arc<dyn Executable>>,
    ) -> bool {
        if group_key == "DEFAULT_KEY" && !executable_list.is_empty() {
            self.item = Arc::clone(&executable_list[0]);
            true
        } else {
            false
        }
    }

    fn condition_type(&self) -> ConditionTypeEnum {
        PreCondition::condition_type(self)
    }
}
