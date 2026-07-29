//! 对应 Java 类：com.yomahub.liteflow.flow.element.condition.ThenCondition
//!
//! 串行器：pre → 主体顺序执行 → finally（必执行）。
//! 异常记入 slot 并向上抛出；ChainEnd 原样上抛。
//!
//! Java 在 finally 块中执行 FinallyCondition：若 FINALLY 自身失败，它会覆盖主体
//! 异常并停止后续 FINALLY；Rust 保留同一异常优先级与停止语义。

use super::{Condition, ConditionBase};
use crate::enums::ConditionTypeEnum;
use crate::exception::{LFResult, LiteflowError};
use crate::flow::element::executable::Executable;
use crate::slot::{Ctx, Frame};
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Clone)]
pub struct ThenCondition {
    base: ConditionBase,
    pre_list: Vec<Arc<dyn Executable>>,
    executable_list: Vec<Arc<dyn Executable>>,
    finally_list: Vec<Arc<dyn Executable>>,
}

impl ThenCondition {
    /// 创建空的串行 Condition。
    ///
    /// 新对象的 PRE、主体和 FINALLY 列表均为空，后续由 EL Builder 通过
    /// `add_executable` 装配。对应 Java: `ThenCondition#ThenCondition`。
    #[must_use]
    pub fn new() -> Self {
        Self {
            base: ConditionBase::default(),
            pre_list: Vec::new(),
            executable_list: Vec::new(),
            finally_list: Vec::new(),
        }
    }
    /// 对应 Java ThenCondition#addExecutable：按类型分流，
    /// PreCondition → pre_list，FinallyCondition → finally_list，其余进主体列表
    pub fn add_executable(&mut self, item: Arc<dyn Executable>) {
        if item.is_pre_or_finally() {
            match item.id() {
                "PRE" => {
                    self.pre_list.push(item);
                    return;
                }
                "FINALLY" => {
                    self.finally_list.push(item);
                    return;
                }
                _ => {}
            }
        }
        self.executable_list.push(item);
    }
    /// 对应 Java ThenCondition#addPreCondition
    pub fn add_pre_condition(&mut self, item: Arc<dyn Executable>) {
        self.pre_list.push(item);
    }
    /// 对应 Java ThenCondition#addFinallyCondition
    pub fn add_finally_condition(&mut self, item: Arc<dyn Executable>) {
        self.finally_list.push(item);
    }
    /// 执行 THEN 条件主体。对应 Java: `ThenCondition#executeCondition`。
    pub async fn execute_condition(&self, ctx: &Ctx, frame: &Frame) -> LFResult<Value> {
        <Self as Executable>::execute(self, ctx, frame).await
    }

    /// 返回前置条件列表。对应 Java: `ThenCondition#getPreConditionList`。
    #[must_use]
    pub fn get_pre_condition_list(&self) -> &[Arc<dyn Executable>] {
        &self.pre_list
    }

    /// 返回后置条件列表。对应 Java: `ThenCondition#getFinallyConditionList`。
    #[must_use]
    pub fn get_finally_condition_list(&self) -> &[Arc<dyn Executable>] {
        &self.finally_list
    }

    /// 返回条件类型。对应 Java: `ThenCondition#getConditionType`。
    #[must_use]
    pub fn get_condition_type(&self) -> ConditionTypeEnum {
        ConditionTypeEnum::Then
    }

    /// 返回条件类型的 Rust 惯用别名。
    pub fn condition_type(&self) -> ConditionTypeEnum {
        self.get_condition_type()
    }
}

#[async_trait]
impl Executable for ThenCondition {
    async fn execute(&self, ctx: &Ctx, frame: &Frame) -> LFResult<Value> {
        // Java Condition 的 bindData 随当前 Condition 压入执行上下文。大多数
        // EL 属性由 BindWrapperCondition 承载；Chain.tag 首先创建的原生
        // ThenCondition 则会直接持有后续 bind，因此这里也必须下传自身绑定。
        let frame = frame.push_bind(self.base.bind_data());
        super::execute_condition_with_lifecycle(self, ctx, &frame, async {
            let mut err: Option<LiteflowError> = None;
            for item in self.pre_list.iter().chain(self.executable_list.iter()) {
                match item.execute(ctx, &frame).await {
                    Ok(_) => {}
                    Err(e) => {
                        err = Some(e);
                        break;
                    }
                }
            }
            // FINALLY 无论主体是否失败都必须执行。Java finally 块中的普通 for
            // 循环在首个 FINALLY 异常处停止，且该异常覆盖 try/catch 正在传播的
            // 主体异常；这里显式替换 err 以保留同一优先级。
            for fin in &self.finally_list {
                if let Err(fe) = fin.execute(ctx, &frame).await {
                    err = Some(fe);
                    break;
                }
            }
            match err {
                Some(LiteflowError::ChainEnd(message)) => Err(LiteflowError::ChainEnd(message)),
                Some(e) => {
                    ctx.set_exception(&e.to_string());
                    Err(e)
                }
                None => Ok(Value::Null),
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
        self.base.explicit_id().unwrap_or("THEN")
    }
}

impl Condition for ThenCondition {
    fn condition_base(&self) -> &ConditionBase {
        &self.base
    }

    fn condition_base_mut(&mut self) -> &mut ConditionBase {
        &mut self.base
    }

    fn typed_executable_group(&self) -> HashMap<String, Vec<Arc<dyn Executable>>> {
        HashMap::from([
            ("DEFAULT_KEY".to_string(), self.executable_list.clone()),
            ("PRE_KEY".to_string(), self.pre_list.clone()),
            ("FINALLY_KEY".to_string(), self.finally_list.clone()),
        ])
    }

    fn replace_typed_executable_group(
        &mut self,
        group_key: &str,
        executable_list: Vec<Arc<dyn Executable>>,
    ) -> bool {
        match group_key {
            "DEFAULT_KEY" => self.executable_list = executable_list,
            "PRE_KEY" => self.pre_list = executable_list,
            "FINALLY_KEY" => self.finally_list = executable_list,
            _ => return false,
        }
        true
    }

    fn condition_type(&self) -> ConditionTypeEnum {
        ThenCondition::condition_type(self)
    }
}
