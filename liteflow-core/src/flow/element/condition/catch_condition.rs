//! 对应 Java 类：com.yomahub.liteflow.flow.element.condition.CatchCondition
//!
//! 捕获执行 DO；无 DO 则异常继续抛出。
//!
//! 差异说明：
//! - Java 在 catch item 为空时抛 CatchErrorException；Rust 端 catch_item 为
//!   非空字段（builder 保证），不存在该运行期分支。
//! - Java 通过 DataBus.getSlot(slotIndex).removeException() 清除 slot 异常；
//!   Rust 端直接复位 Slot.exception（pub 字段），语义一致。

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
pub struct CatchCondition {
    base: ConditionBase,
    catch_item: Arc<dyn Executable>,
    do_item: Option<Arc<dyn Executable>>,
}

impl CatchCondition {
    /// 使用被捕获执行项和可选恢复执行项创建 CATCH 条件。
    ///
    /// 对应 Java `CATCH_KEY` 与 `DO_KEY` 分组。
    pub fn new(catch_item: Arc<dyn Executable>, do_item: Option<Arc<dyn Executable>>) -> Self {
        Self {
            base: ConditionBase::default(),
            catch_item,
            do_item,
        }
    }

    /// 执行 CATCH 条件主体。对应 Java: `CatchCondition#executeCondition`。
    pub async fn execute_condition(&self, ctx: &Ctx, frame: &Frame) -> LFResult<Value> {
        <Self as Executable>::execute(self, ctx, frame).await
    }

    /// 返回条件类型。对应 Java: `CatchCondition#getConditionType`。
    #[must_use]
    pub fn get_condition_type(&self) -> ConditionTypeEnum {
        ConditionTypeEnum::Catch
    }

    /// 返回被捕获执行项。对应 Java: `CatchCondition#getCatchItem`。
    #[must_use]
    pub fn get_catch_item(&self) -> &Arc<dyn Executable> {
        &self.catch_item
    }

    /// 设置被捕获执行项。
    ///
    /// - `executable`: 其异常将被 CATCH 处理的对象。
    ///
    /// 对应 Java: `CatchCondition#setCatchItem`。
    pub fn set_catch_item(&mut self, executable: Arc<dyn Executable>) {
        self.catch_item = executable;
    }

    /// 返回异常恢复执行项。对应 Java: `CatchCondition#getDoItem`。
    #[must_use]
    pub fn get_do_item(&self) -> Option<&Arc<dyn Executable>> {
        self.do_item.as_ref()
    }

    /// 设置异常恢复执行项。
    ///
    /// - `executable`: 捕获异常后执行的 DO 对象。
    ///
    /// 对应 Java: `CatchCondition#setDoItem`。
    pub fn set_do_item(&mut self, executable: Arc<dyn Executable>) {
        self.do_item = Some(executable);
    }

    /// 返回条件类型的 Rust 惯用别名。
    pub fn condition_type(&self) -> ConditionTypeEnum {
        self.get_condition_type()
    }
}

#[async_trait]
impl Executable for CatchCondition {
    async fn execute(&self, ctx: &Ctx, frame: &Frame) -> LFResult<Value> {
        super::execute_condition_with_lifecycle(self, ctx, frame, async {
            match self.catch_item.execute(ctx, frame).await {
                Ok(v) => Ok(v),
                Err(LiteflowError::ChainEnd(message)) => Err(LiteflowError::ChainEnd(message)),
                Err(e) => {
                    ctx.set_exception(&e.to_string());
                    match &self.do_item {
                        Some(d) => {
                            let r = d.execute(ctx, frame).await;
                            if r.is_ok() {
                                // 对应 Java CatchCondition#executeCondition：
                                // catch 之后需要把 exception 清除掉——正如同 java 的 catch，
                                // 异常自己处理了属于正常流程，整个流程状态应该是成功的
                                if let Ok(mut ex) = ctx.inner.exception.lock() {
                                    *ex = None;
                                }
                            }
                            r
                        }
                        None => Err(e),
                    }
                }
            }
        })
        .await
    }

    fn collect_node_ids(&self) -> Vec<String> {
        Condition::get_all_node_in_condition(self)
    }

    fn id(&self) -> &str {
        "CATCH"
    }
}

impl Condition for CatchCondition {
    fn condition_base(&self) -> &ConditionBase {
        &self.base
    }

    fn condition_base_mut(&mut self) -> &mut ConditionBase {
        &mut self.base
    }

    fn typed_executable_group(&self) -> HashMap<String, Vec<Arc<dyn Executable>>> {
        let mut groups =
            HashMap::from([("CATCH_KEY".to_string(), vec![Arc::clone(&self.catch_item)])]);
        if let Some(do_item) = &self.do_item {
            groups.insert("DO_KEY".to_string(), vec![Arc::clone(do_item)]);
        }
        groups
    }

    fn replace_typed_executable_group(
        &mut self,
        group_key: &str,
        executable_list: Vec<Arc<dyn Executable>>,
    ) -> bool {
        match group_key {
            "CATCH_KEY" if !executable_list.is_empty() => {
                self.catch_item = Arc::clone(&executable_list[0]);
                true
            }
            "DO_KEY" => {
                self.do_item = executable_list.into_iter().next();
                true
            }
            _ => false,
        }
    }

    fn condition_type(&self) -> ConditionTypeEnum {
        CatchCondition::condition_type(self)
    }
}
