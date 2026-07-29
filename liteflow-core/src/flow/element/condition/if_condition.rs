//! 对应 Java 类：com.yomahub.liteflow.flow.element.condition.IfCondition
//!
//! 条件结果驱动 true/false 分支（含 ELIF）。
//! isAccess=false 直接返回；true/false 目标不可为 pre/finally。
//!
//! 差异说明：
//! - Java 先校验 if 节点类型为 IF/IF_SCRIPT（否则抛 IfTypeErrorException）；
//!   Rust 端节点类型由 builder 保证，结果非 bool 时报 NodeTypeError（同语义）。
//! - Java 通过 slot.getIfResult(类名) 取条件结果；Rust 端条件节点直接返回
//!   Value::Bool（见 crate::flow::element::node）。

use super::{Condition, ConditionBase, check_not_pre_finally, expect_bool};
use crate::enums::ConditionTypeEnum;
use crate::exception::{LFResult, LiteflowError};
use crate::flow::element::executable::Executable;
use crate::slot::{Ctx, Frame};
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Clone)]
pub struct IfCondition {
    base: ConditionBase,
    if_item: Arc<dyn Executable>,
    true_case: Option<Arc<dyn Executable>>,
    /// (elif 条件, elif 目标)
    elif_list: Vec<(Arc<dyn Executable>, Arc<dyn Executable>)>,
    false_case: Option<Arc<dyn Executable>>,
}

impl IfCondition {
    /// 使用 IF 判定项、真假分支、ELIF 分支创建条件。
    ///
    /// 参数分别对应 Java `IF_KEY`、`IF_TRUE_CASE_KEY`、ELIF 列表与
    /// `IF_FALSE_CASE_KEY`。对应 Java: `IfCondition` 的分组装配过程。
    pub fn new(
        if_item: Arc<dyn Executable>,
        true_case: Arc<dyn Executable>,
        elif_list: Vec<(Arc<dyn Executable>, Arc<dyn Executable>)>,
        false_case: Option<Arc<dyn Executable>>,
    ) -> Self {
        Self {
            base: ConditionBase::default(),
            if_item,
            true_case: Some(true_case),
            elif_list,
            false_case,
        }
    }

    /// 执行 IF 条件主体。
    ///
    /// - `ctx`: 当前 Slot 上下文。
    /// - `frame`: 当前任务执行帧。
    ///
    /// 对应 Java: `IfCondition#executeCondition`。
    pub async fn execute_condition(&self, ctx: &Ctx, frame: &Frame) -> LFResult<Value> {
        <Self as Executable>::execute(self, ctx, frame).await
    }

    /// 返回条件类型。对应 Java: `IfCondition#getConditionType`。
    #[must_use]
    pub fn get_condition_type(&self) -> ConditionTypeEnum {
        ConditionTypeEnum::If
    }

    /// 返回 true 分支可执行项。对应 Java: `IfCondition#getTrueCaseExecutableItem`。
    #[must_use]
    pub fn get_true_case_executable_item(&self) -> Option<&Arc<dyn Executable>> {
        self.true_case.as_ref()
    }

    /// 设置 true 分支可执行项。
    ///
    /// - `true_case_executable_item`: IF 为 true 时执行的目标。
    ///
    /// 对应 Java: `IfCondition#setTrueCaseExecutableItem`。
    pub fn set_true_case_executable_item(
        &mut self,
        true_case_executable_item: Arc<dyn Executable>,
    ) {
        self.true_case = Some(true_case_executable_item);
    }

    /// 返回 false 分支可执行项。对应 Java: `IfCondition#getFalseCaseExecutableItem`。
    #[must_use]
    pub fn get_false_case_executable_item(&self) -> Option<&Arc<dyn Executable>> {
        self.false_case.as_ref()
    }

    /// 设置 false 分支可执行项。
    ///
    /// - `false_case_executable_item`: IF 为 false 时执行的目标。
    ///
    /// 对应 Java: `IfCondition#setFalseCaseExecutableItem`。
    pub fn set_false_case_executable_item(
        &mut self,
        false_case_executable_item: Arc<dyn Executable>,
    ) {
        self.false_case = Some(false_case_executable_item);
    }

    /// 设置 IF 判定项。
    ///
    /// - `if_item`: 必须返回布尔值的判定可执行项。
    ///
    /// 对应 Java: `IfCondition#setIfItem`。
    pub fn set_if_item(&mut self, if_item: Arc<dyn Executable>) {
        self.if_item = if_item;
    }

    /// 返回 IF 判定项。对应 Java: `IfCondition#getIfItem`。
    #[must_use]
    pub fn get_if_item(&self) -> &Arc<dyn Executable> {
        &self.if_item
    }

    /// 返回条件类型的 Rust 惯用别名。
    pub fn condition_type(&self) -> ConditionTypeEnum {
        self.get_condition_type()
    }
}

#[async_trait]
impl Executable for IfCondition {
    async fn execute(&self, ctx: &Ctx, frame: &Frame) -> LFResult<Value> {
        super::execute_condition_with_lifecycle(self, ctx, frame, async {
            // 对应 Java IfCondition#executeCondition：先判断 isAccess，
            // 返回 false 则整个 IF 表达式不执行
            if !self.if_item.is_access(ctx, frame).await {
                return Ok(Value::Null);
            }
            let v = self.if_item.execute(ctx, frame).await?;
            if expect_bool(self.if_item.id(), &v)? {
                let true_case = self
                    .true_case
                    .as_ref()
                    .ok_or_else(|| LiteflowError::NoIfTrueNode(self.if_item.id().to_string()))?;
                check_not_pre_finally(true_case.as_ref(), self.if_item.id())?;
                return true_case.execute(ctx, frame).await;
            }
            for (c, t) in &self.elif_list {
                // Java ELIF 通过嵌套 IfCondition 表达；内层判定项不可访问时，
                // 内层 IF 立即返回，不能继续进入后续 ELIF/ELSE。
                if !c.is_access(ctx, frame).await {
                    return Ok(Value::Null);
                }
                let v = c.execute(ctx, frame).await?;
                if expect_bool(c.id(), &v)? {
                    check_not_pre_finally(t.as_ref(), self.if_item.id())?;
                    return t.execute(ctx, frame).await;
                }
            }
            if let Some(f) = &self.false_case {
                check_not_pre_finally(f.as_ref(), self.if_item.id())?;
                return f.execute(ctx, frame).await;
            }
            Ok(Value::Null)
        })
        .await
    }

    fn collect_node_ids(&self) -> Vec<String> {
        Condition::get_all_node_in_condition(self)
    }

    fn apply_chain_cmp_data(&self, data: &str) {
        super::apply_chain_cmp_data_to_condition(self, data);
    }

    fn id(&self) -> &str {
        "IF"
    }
}

impl Condition for IfCondition {
    fn condition_base(&self) -> &ConditionBase {
        &self.base
    }

    fn condition_base_mut(&mut self) -> &mut ConditionBase {
        &mut self.base
    }

    fn typed_executable_group(&self) -> HashMap<String, Vec<Arc<dyn Executable>>> {
        let mut if_items = vec![Arc::clone(&self.if_item)];
        let mut true_items = self.true_case.iter().cloned().collect::<Vec<_>>();
        for (elif_condition, elif_target) in &self.elif_list {
            if_items.push(Arc::clone(elif_condition));
            true_items.push(Arc::clone(elif_target));
        }
        let mut groups = HashMap::from([
            ("IF_KEY".to_string(), if_items),
            ("IF_TRUE_CASE_KEY".to_string(), true_items),
        ]);
        if let Some(false_case) = &self.false_case {
            groups.insert(
                "IF_FALSE_CASE_KEY".to_string(),
                vec![Arc::clone(false_case)],
            );
        }
        groups
    }

    fn replace_typed_executable_group(
        &mut self,
        group_key: &str,
        executable_list: Vec<Arc<dyn Executable>>,
    ) -> bool {
        match group_key {
            "IF_KEY" if !executable_list.is_empty() => {
                self.if_item = Arc::clone(&executable_list[0]);
                for (index, executable) in executable_list.into_iter().skip(1).enumerate() {
                    if let Some((condition, _)) = self.elif_list.get_mut(index) {
                        *condition = executable;
                    }
                }
                true
            }
            "IF_TRUE_CASE_KEY" => {
                let mut executable_iter = executable_list.into_iter();
                self.true_case = executable_iter.next();
                for (index, executable) in executable_iter.enumerate() {
                    if let Some((_, target)) = self.elif_list.get_mut(index) {
                        *target = executable;
                    }
                }
                true
            }
            "IF_FALSE_CASE_KEY" => {
                self.false_case = executable_list.into_iter().next();
                true
            }
            _ => false,
        }
    }

    fn condition_type(&self) -> ConditionTypeEnum {
        IfCondition::condition_type(self)
    }
}
