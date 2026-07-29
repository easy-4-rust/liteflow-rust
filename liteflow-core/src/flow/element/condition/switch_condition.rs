//! 对应 Java 类：com.yomahub.liteflow.flow.element.condition.SwitchCondition
//!
//! "id:tag" 目标匹配规则、default、NoSwitchTargetNodeException。
//!
//! 差异说明：
//! - Java 先校验 switch 节点类型为 SWITCH/SWITCH_SCRIPT（否则抛
//!   SwitchTypeErrorException）；Rust 端节点类型由 builder 保证，结果非
//!   string 时报 NodeTypeError（同语义）。
//! - Java 通过 slot.getSwitchResult(类名) 取选择结果；Rust 端 switch 节点
//!   直接返回 Value::String。

use super::{Condition, ConditionBase, check_not_pre_finally};
use crate::enums::ConditionTypeEnum;
use crate::exception::{LFResult, LiteflowError};
use crate::flow::element::executable::Executable;
use crate::slot::{Ctx, Frame};
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Clone)]
pub struct SwitchCondition {
    base: ConditionBase,
    switch_node: Arc<dyn Executable>,
    target_list: Vec<Arc<dyn Executable>>,
    default_executor: Option<Arc<dyn Executable>>,
}

impl SwitchCondition {
    /// 使用路由节点、候选目标和默认目标创建 SWITCH 条件。
    ///
    /// 参数分别对应 Java `SwitchCondition` 的 `SWITCH_KEY`、
    /// `SWITCH_TARGET_KEY` 与 `SWITCH_DEFAULT_KEY` 可执行对象组。
    pub fn new(
        switch_node: Arc<dyn Executable>,
        target_list: Vec<Arc<dyn Executable>>,
        default_executor: Option<Arc<dyn Executable>>,
    ) -> Self {
        Self {
            base: ConditionBase::default(),
            switch_node,
            target_list,
            default_executor,
        }
    }

    /// 返回 SWITCH 的候选目标可执行对象。
    ///
    /// 返回值对应 Java `SwitchCondition#getTargetList`。
    #[must_use]
    pub fn get_target_list(&self) -> &[Arc<dyn Executable>] {
        &self.target_list
    }

    /// 执行 SWITCH 条件主体。
    ///
    /// 对应 Java: `SwitchCondition#executeCondition`。
    pub async fn execute_condition(&self, ctx: &Ctx, frame: &Frame) -> LFResult<Value> {
        <Self as Executable>::execute(self, ctx, frame).await
    }

    /// 返回条件类型。对应 Java: `SwitchCondition#getConditionType`。
    #[must_use]
    pub fn get_condition_type(&self) -> ConditionTypeEnum {
        ConditionTypeEnum::Switch
    }

    /// 添加一个候选目标。
    ///
    /// - `executable`: SWITCH 结果可以选择的目标。
    ///
    /// 对应 Java: `SwitchCondition#addTargetItem`。
    pub fn add_target_item(&mut self, executable: Arc<dyn Executable>) {
        self.target_list.push(executable);
    }

    /// 设置 SWITCH 路由节点。
    ///
    /// - `switch_node`: 返回目标 ID 或 `id:tag` 的可执行项。
    ///
    /// 对应 Java: `SwitchCondition#setSwitchNode`。
    pub fn set_switch_node(&mut self, switch_node: Arc<dyn Executable>) {
        self.switch_node = switch_node;
    }

    /// 返回 SWITCH 路由节点。对应 Java: `SwitchCondition#getSwitchNode`。
    #[must_use]
    pub fn get_switch_node(&self) -> &Arc<dyn Executable> {
        &self.switch_node
    }

    /// 返回默认目标。对应 Java: `SwitchCondition#getDefaultExecutor`。
    #[must_use]
    pub fn get_default_executor(&self) -> Option<&Arc<dyn Executable>> {
        self.default_executor.as_ref()
    }

    /// 设置默认目标。
    ///
    /// - `default_executor`: 未匹配候选目标时执行的对象。
    ///
    /// 对应 Java: `SwitchCondition#setDefaultExecutor`。
    pub fn set_default_executor(&mut self, default_executor: Arc<dyn Executable>) {
        self.default_executor = Some(default_executor);
    }

    /// 返回条件类型的 Rust 惯用别名。
    #[must_use]
    pub fn condition_type(&self) -> ConditionTypeEnum {
        self.get_condition_type()
    }
}

#[async_trait]
impl Executable for SwitchCondition {
    async fn execute(&self, ctx: &Ctx, frame: &Frame) -> LFResult<Value> {
        super::execute_condition_with_lifecycle(self, ctx, frame, async {
            // Java 在 Condition#execute 中先把当前 SwitchCondition 压入 Slot；
            // Rust 把目标 ID 放入不可变 Frame，使路由组件可读取同一条件上下文。
            let target_ids = self
                .get_target_list()
                .iter()
                .map(|target| target.id().to_string())
                .collect::<Vec<_>>();
            let switch_frame = frame.with_switch_target_list(&target_ids);

            // 对应 Java SwitchCondition#executeCondition：先判断 isAccess，
            // 返回 false 则整个 SWITCH 表达式不执行
            if !self.switch_node.is_access(ctx, &switch_frame).await {
                return Ok(Value::Null);
            }
            let v = self.switch_node.execute(ctx, &switch_frame).await?;
            let target_id = match &v {
                Value::String(s) => s.clone(),
                Value::Null => String::new(),
                other => {
                    return Err(LiteflowError::NodeTypeError {
                        node: self.switch_node.id().to_string(),
                        expect: "string".into(),
                        actual: other.to_string(),
                    });
                }
            };

            let mut target: Option<&Arc<dyn Executable>> = None;
            // Java StrUtil.isNotBlank：纯空白路由值不参与 ID/标签匹配，
            // 而是直接进入 DEFAULT 分支。
            if !target_id.trim().is_empty() {
                // 对齐 Java 的 tag 匹配规则："id:tag" / ":tag" / "id"
                if target_id.contains(':') {
                    let mut parts = target_id.splitn(2, ':');
                    let tid = parts.next().unwrap_or("");
                    let ttag = parts.next().unwrap_or("");
                    target = self.target_list.iter().find(|e| {
                        (tid.starts_with("tag") && e.tag() == Some(ttag))
                            || ((tid.is_empty() || tid == e.id())
                                && (ttag.is_empty() || e.tag() == Some(ttag)))
                    });
                } else {
                    target = self.target_list.iter().find(|e| e.id() == target_id);
                }
            }
            let target = target.or(self.default_executor.as_ref());
            match target {
                Some(t) => {
                    check_not_pre_finally(t.as_ref(), self.switch_node.id())?;
                    t.execute(ctx, &switch_frame).await
                }
                None => Err(LiteflowError::NoSwitchTarget(target_id)),
            }
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
        "SWITCH"
    }
}

impl Condition for SwitchCondition {
    fn condition_base(&self) -> &ConditionBase {
        &self.base
    }

    fn condition_base_mut(&mut self) -> &mut ConditionBase {
        &mut self.base
    }

    fn typed_executable_group(&self) -> HashMap<String, Vec<Arc<dyn Executable>>> {
        let mut groups = HashMap::from([
            (
                "SWITCH_KEY".to_string(),
                vec![Arc::clone(&self.switch_node)],
            ),
            ("SWITCH_TARGET_KEY".to_string(), self.target_list.clone()),
        ]);
        if let Some(default_executor) = &self.default_executor {
            groups.insert(
                "SWITCH_DEFAULT_KEY".to_string(),
                vec![Arc::clone(default_executor)],
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
            "SWITCH_KEY" if !executable_list.is_empty() => {
                self.switch_node = Arc::clone(&executable_list[0]);
                true
            }
            "SWITCH_TARGET_KEY" => {
                self.target_list = executable_list;
                true
            }
            "SWITCH_DEFAULT_KEY" => {
                self.default_executor = executable_list.into_iter().next();
                true
            }
            _ => false,
        }
    }

    fn condition_type(&self) -> ConditionTypeEnum {
        SwitchCondition::condition_type(self)
    }
}
