//! 对应 Java 类：com.yomahub.liteflow.flow.element.condition.AndOrCondition
//!
//! AND/OR 布尔短路，并把结果保存到当前任务 Frame。

use super::{Condition, ConditionBase, expect_bool};
use crate::enums::ConditionTypeEnum;
use crate::exception::LFResult;
use crate::flow::element::condition::BooleanConditionTypeEnum;
use crate::flow::element::executable::Executable;
use crate::slot::{Ctx, Frame};
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Clone)]
pub struct AndOrCondition {
    base: ConditionBase,
    condition_type: BooleanConditionTypeEnum,
    items: Vec<Arc<dyn Executable>>,
}

/// AND/OR 子项谓词。
///
/// 这是 Java `AndOrCondition.AndOrConditionPredicate` 内部类的 Rust 伴随类型，
/// 与主对象保留在同一文件。它执行单个子项并把结果校验为布尔值。
pub struct AndOrConditionPredicate<'a> {
    ctx: &'a Ctx,
    frame: &'a Frame,
}

impl AndOrConditionPredicate<'_> {
    /// 执行并判断一个 AND/OR 子项。
    ///
    /// - `executable`: 已通过 `is_access` 过滤的布尔可执行项。
    ///
    /// 对应 Java: `AndOrConditionPredicate#test`。
    pub async fn test(&self, executable: &dyn Executable) -> LFResult<bool> {
        let value = executable.execute(self.ctx, self.frame).await?;
        expect_bool(executable.id(), &value)
    }
}

impl AndOrCondition {
    /// 使用布尔组合类型和子项创建条件。
    ///
    /// 对应 Java `setBooleanConditionType` 与 `addItem` 完成后的对象状态。
    pub fn new(condition_type: BooleanConditionTypeEnum, items: Vec<Arc<dyn Executable>>) -> Self {
        Self {
            base: ConditionBase::default(),
            condition_type,
            items,
        }
    }

    fn result_key(&self) -> String {
        format!("AndOrCondition_{:p}", self)
    }

    /// 创建绑定当前执行上下文的子项谓词。
    ///
    /// 对应 Java 内部类构造器 `AndOrConditionPredicate#AndOrConditionPredicate`。
    #[must_use]
    pub fn and_or_condition_predicate<'a>(
        &self,
        ctx: &'a Ctx,
        frame: &'a Frame,
    ) -> AndOrConditionPredicate<'a> {
        AndOrConditionPredicate { ctx, frame }
    }

    /// 执行 AND/OR 条件主体。对应 Java: `AndOrCondition#executeCondition`。
    pub async fn execute_condition(&self, ctx: &Ctx, frame: &Frame) -> LFResult<Value> {
        <Self as Executable>::execute(self, ctx, frame).await
    }

    /// 返回最近一次执行结果。
    ///
    /// 尚未执行时返回 `None`。对应 Java:
    /// `AndOrCondition#getItemResultMetaValue`。
    #[must_use]
    pub fn get_item_result_meta_value(&self, frame: &Frame) -> Option<bool> {
        frame.get_and_or_result(&self.result_key())
    }

    /// 返回条件类型。对应 Java: `AndOrCondition#getConditionType`。
    #[must_use]
    pub fn get_condition_type(&self) -> ConditionTypeEnum {
        ConditionTypeEnum::AndOr
    }

    /// 添加布尔子项。对应 Java: `AndOrCondition#addItem`。
    pub fn add_item(&mut self, item: Arc<dyn Executable>) {
        self.items.push(item);
    }

    /// 返回全部布尔子项。对应 Java: `AndOrCondition#getItem`。
    #[must_use]
    pub fn get_item(&self) -> &[Arc<dyn Executable>] {
        &self.items
    }

    /// 返回 AND/OR 类型。
    ///
    /// 对应 Java: `AndOrCondition#getBooleanConditionType`。
    #[must_use]
    pub fn get_boolean_condition_type(&self) -> BooleanConditionTypeEnum {
        self.condition_type
    }

    /// 设置 AND/OR 类型。
    ///
    /// 对应 Java: `AndOrCondition#setBooleanConditionType`。
    pub fn set_boolean_condition_type(&mut self, boolean_condition_type: BooleanConditionTypeEnum) {
        self.condition_type = boolean_condition_type;
    }
}

#[async_trait]
impl Executable for AndOrCondition {
    async fn execute(&self, ctx: &Ctx, frame: &Frame) -> LFResult<Value> {
        super::execute_condition_with_lifecycle(self, ctx, frame, async {
            if self.items.is_empty() {
                return Err(crate::exception::LiteflowError::AndOrCondition(
                    "boolean item list is null".to_string(),
                ));
            }
            // 2.16 语义：先按 isAccess 过滤（不可访问/异常的子项被排除），
            // 再对剩余子项做 allMatch / anyMatch
            let mut accessible = Vec::with_capacity(self.items.len());
            for item in &self.items {
                if item.is_access(ctx, frame).await {
                    accessible.push(item);
                }
            }
            let predicate = self.and_or_condition_predicate(ctx, frame);
            let result = match self.condition_type {
                BooleanConditionTypeEnum::And => {
                    for item in accessible {
                        if !predicate.test(item.as_ref()).await? {
                            frame.set_and_or_result(self.result_key(), false);
                            return Ok(Value::Bool(false));
                        }
                    }
                    true
                }
                BooleanConditionTypeEnum::Or => {
                    for item in accessible {
                        if predicate.test(item.as_ref()).await? {
                            frame.set_and_or_result(self.result_key(), true);
                            return Ok(Value::Bool(true));
                        }
                    }
                    false
                }
            };
            frame.set_and_or_result(self.result_key(), result);
            Ok(Value::Bool(result))
        })
        .await
    }

    fn collect_node_ids(&self) -> Vec<String> {
        Condition::get_all_node_in_condition(self)
    }

    fn id(&self) -> &str {
        match self.condition_type {
            BooleanConditionTypeEnum::And => "AND",
            BooleanConditionTypeEnum::Or => "OR",
        }
    }
}

impl Condition for AndOrCondition {
    fn condition_base(&self) -> &ConditionBase {
        &self.base
    }

    fn condition_base_mut(&mut self) -> &mut ConditionBase {
        &mut self.base
    }

    fn typed_executable_group(&self) -> HashMap<String, Vec<Arc<dyn Executable>>> {
        HashMap::from([("DEFAULT_KEY".to_string(), self.items.clone())])
    }

    fn replace_typed_executable_group(
        &mut self,
        group_key: &str,
        executable_list: Vec<Arc<dyn Executable>>,
    ) -> bool {
        if group_key == "DEFAULT_KEY" {
            self.items = executable_list;
            true
        } else {
            false
        }
    }

    fn condition_type(&self) -> ConditionTypeEnum {
        self.get_condition_type()
    }
}
