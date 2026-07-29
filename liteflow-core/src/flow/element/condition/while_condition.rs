//! 对应 Java 类：com.yomahub.liteflow.flow.element.condition.WhileCondition
//!
//! 条件循环及 Java PARALLEL 布尔并行形态。
//!
//! 差异说明：
//! - Java 在 whileNode 为空时抛 NoWhileNodeException；Rust 端 while_item 为
//!   非空字段（builder 保证），不存在该运行期分支。
//! - Java 通过 slot.getWhileResult(类名) 取循环条件；Rust 端 while 节点直接
//!   返回 Value::Bool。

use super::loop_condition::{LoopCondition, handle_future_list, run_sequential, submit_iteration};
use super::{Condition, ConditionBase, expect_bool};
use crate::enums::ConditionTypeEnum;
use crate::exception::LFResult;
use crate::flow::element::executable::Executable;
use crate::flow::parallel::LoopFutureObj;
use crate::slot::{Ctx, Frame};
use crate::thread::ExecutorHelper;
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::task::JoinSet;

#[derive(Clone)]
pub struct WhileCondition {
    base: ConditionBase,
    while_item: Arc<dyn Executable>,
    pub parallel: bool,
    thread_pool_executor_class: Option<String>,
    do_executor: Arc<dyn Executable>,
    break_item: Option<Arc<dyn Executable>>,
}

impl WhileCondition {
    /// 使用循环判定项、并行配置、循环体和 BREAK 项创建 WHILE 条件。
    ///
    /// 对应 Java `WHILE_KEY` 与 `LoopCondition` 公共字段的装配结果。
    pub fn new(
        while_item: Arc<dyn Executable>,
        parallel: bool,
        do_executor: Arc<dyn Executable>,
        break_item: Option<Arc<dyn Executable>>,
    ) -> Self {
        Self {
            base: ConditionBase::default(),
            while_item,
            parallel,
            thread_pool_executor_class: None,
            do_executor,
            break_item,
        }
    }

    /// 执行 WHILE 循环。对应 Java: `WhileCondition#executeCondition`。
    pub async fn execute_condition(&self, ctx: &Ctx, frame: &Frame) -> LFResult<Value> {
        <Self as Executable>::execute(self, ctx, frame).await
    }

    /// 返回条件类型。对应 Java: `WhileCondition#getConditionType`。
    #[must_use]
    pub fn get_condition_type(&self) -> ConditionTypeEnum {
        ConditionTypeEnum::While
    }

    /// 返回循环判定项。对应 Java: `WhileCondition#getWhileItem`。
    #[must_use]
    pub fn get_while_item(&self) -> &Arc<dyn Executable> {
        &self.while_item
    }

    /// 设置循环判定项。
    ///
    /// - `while_item`: 每轮执行且必须返回布尔值的可执行项。
    ///
    /// 对应 Java: `WhileCondition#setWhileItem`。
    pub fn set_while_item(&mut self, while_item: Arc<dyn Executable>) {
        self.while_item = while_item;
    }

    /// 返回条件类型的 Rust 惯用别名。
    pub fn condition_type(&self) -> ConditionTypeEnum {
        self.get_condition_type()
    }
}

#[async_trait]
impl Executable for WhileCondition {
    async fn execute(&self, ctx: &Ctx, frame: &Frame) -> LFResult<Value> {
        super::execute_condition_with_lifecycle(self, ctx, frame, async {
            // 对应 Java WhileCondition#executeCondition：先判断 isAccess，
            // 返回 false 则整个 WHILE 表达式不执行
            if !self.while_item.is_access(ctx, frame).await {
                return Ok(Value::Null);
            }
            let mut index = 0usize;
            if self.parallel {
                let condition_key = format!("{:p}", self);
                let executor_service = ExecutorHelper::load_instance().build_executor_service(
                    self.thread_pool_executor_class
                        .as_deref()
                        .or(frame.condition_thread_pool()),
                    frame.chain_thread_pool(),
                    &condition_key,
                    &ctx.inner.chain_id,
                    ConditionTypeEnum::While,
                )?;
                let mut set: JoinSet<LoopFutureObj> = JoinSet::new();
                loop {
                    let f = frame.push(index, None);
                    let v = self.while_item.execute(ctx, &f).await?;
                    if !expect_bool(self.while_item.id(), &v)? {
                        break;
                    }
                    let should_continue = submit_iteration(
                        self,
                        &mut set,
                        &self.do_executor,
                        self.break_item.as_ref(),
                        ctx,
                        frame,
                        index,
                        None,
                        &executor_service,
                    )
                    .await?;
                    if !should_continue {
                        break;
                    }
                    index += 1;
                }
                return handle_future_list(set).await;
            }

            loop {
                let f = frame.push(index, None);
                let v = self.while_item.execute(ctx, &f).await?;
                if !expect_bool(self.while_item.id(), &v)? {
                    break;
                }
                let should_continue = run_sequential(
                    &self.do_executor,
                    self.break_item.as_ref(),
                    ctx,
                    frame,
                    index,
                    None,
                )
                .await?;
                if !should_continue {
                    break;
                }
                index += 1;
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
        "WHILE"
    }
}

impl LoopCondition for WhileCondition {
    fn get_break_item(&self) -> Option<&Arc<dyn Executable>> {
        self.break_item.as_ref()
    }

    fn set_break_item(&mut self, break_item: Arc<dyn Executable>) {
        self.break_item = Some(break_item);
    }

    fn get_do_executor(&self) -> &Arc<dyn Executable> {
        &self.do_executor
    }

    fn set_do_executor(&mut self, executable: Arc<dyn Executable>) {
        self.do_executor = executable;
    }

    fn get_thread_pool_executor_class(&self) -> Option<&str> {
        self.thread_pool_executor_class.as_deref()
    }

    fn set_thread_pool_executor_class(&mut self, thread_pool_executor_class: impl Into<String>) {
        self.thread_pool_executor_class = Some(thread_pool_executor_class.into());
    }

    fn is_parallel(&self) -> bool {
        self.parallel
    }

    fn set_parallel(&mut self, parallel: bool) {
        self.parallel = parallel;
    }
}

impl Condition for WhileCondition {
    fn condition_base(&self) -> &ConditionBase {
        &self.base
    }

    fn condition_base_mut(&mut self) -> &mut ConditionBase {
        &mut self.base
    }

    fn typed_executable_group(&self) -> HashMap<String, Vec<Arc<dyn Executable>>> {
        let mut groups = HashMap::from([
            ("WHILE_KEY".to_string(), vec![Arc::clone(&self.while_item)]),
            ("DO_KEY".to_string(), vec![Arc::clone(&self.do_executor)]),
        ]);
        if let Some(break_item) = &self.break_item {
            groups.insert("BREAK_KEY".to_string(), vec![Arc::clone(break_item)]);
        }
        groups
    }

    fn replace_typed_executable_group(
        &mut self,
        group_key: &str,
        executable_list: Vec<Arc<dyn Executable>>,
    ) -> bool {
        match group_key {
            "WHILE_KEY" if !executable_list.is_empty() => {
                self.while_item = Arc::clone(&executable_list[0]);
                true
            }
            "DO_KEY" if !executable_list.is_empty() => {
                self.do_executor = Arc::clone(&executable_list[0]);
                true
            }
            "BREAK_KEY" => {
                self.break_item = executable_list.into_iter().next();
                true
            }
            _ => false,
        }
    }

    fn condition_type(&self) -> ConditionTypeEnum {
        WhileCondition::condition_type(self)
    }
}
