//! 对应 Java 类：com.yomahub.liteflow.flow.element.condition.NotCondition
//!
//! 执行一个布尔可执行项，将取反后的结果写入当前 Frame 的 NOT 结果区。

use super::{Condition, ConditionBase, expect_bool};
use crate::enums::ConditionTypeEnum;
use crate::exception::LFResult;
use crate::flow::element::executable::Executable;
use crate::slot::{Ctx, Frame};
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Clone)]
pub struct NotCondition {
    base: ConditionBase,
    item: Arc<dyn Executable>,
}

impl NotCondition {
    /// 使用待取反可执行项创建 NOT 条件。
    ///
    /// 对应 Java `ConditionKey.NOT_ITEM_KEY` 的装配结果。
    pub fn new(item: Arc<dyn Executable>) -> Self {
        Self {
            base: ConditionBase::default(),
            item,
        }
    }

    fn result_key(&self) -> String {
        format!("NotCondition_{:p}", self)
    }

    /// 执行 NOT 条件主体。对应 Java: `NotCondition#executeCondition`。
    pub async fn execute_condition(&self, ctx: &Ctx, frame: &Frame) -> LFResult<Value> {
        <Self as Executable>::execute(self, ctx, frame).await
    }

    /// 返回本条件最近一次执行保存的布尔结果。
    ///
    /// 未执行时与 Java `BooleanUtil.isTrue(null)` 一致返回 `false`。
    /// 对应 Java: `NotCondition#getItemResultMetaValue`。
    #[must_use]
    pub fn get_item_result_meta_value(&self, frame: &Frame) -> bool {
        frame.get_not_result(&self.result_key()).unwrap_or(false)
    }

    /// 返回条件类型。对应 Java: `NotCondition#getConditionType`。
    #[must_use]
    pub fn get_condition_type(&self) -> ConditionTypeEnum {
        ConditionTypeEnum::Not
    }

    /// 设置待取反可执行项。对应 Java: `NotCondition#setItem`。
    pub fn set_item(&mut self, item: Arc<dyn Executable>) {
        self.item = item;
    }

    /// 返回待取反可执行项。对应 Java: `NotCondition#getItem`。
    #[must_use]
    pub fn get_item(&self) -> &Arc<dyn Executable> {
        &self.item
    }
}

#[async_trait]
impl Executable for NotCondition {
    async fn execute(&self, ctx: &Ctx, frame: &Frame) -> LFResult<Value> {
        super::execute_condition_with_lifecycle(self, ctx, frame, async {
            let v = self.item.execute(ctx, frame).await?;
            let result = !expect_bool(self.item.id(), &v)?;
            frame.set_not_result(self.result_key(), result);
            Ok(Value::Bool(result))
        })
        .await
    }

    fn collect_node_ids(&self) -> Vec<String> {
        Condition::get_all_node_in_condition(self)
    }

    fn id(&self) -> &str {
        "NOT"
    }
}

impl Condition for NotCondition {
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
        self.get_condition_type()
    }
}
