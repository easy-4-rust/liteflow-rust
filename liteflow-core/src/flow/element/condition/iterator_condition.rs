//! 对应 Java 类：com.yomahub.liteflow.flow.element.condition.IteratorCondition
//!
//! 迭代循环，loopObject 传递（含并行形态，PARALLEL 为 Rust 端扩展形态）。
//!
//! 差异说明：
//! - Java 在 iteratorNode 为空时抛 NoIteratorNodeException；Rust 端
//!   iterator_node 为非空字段（builder 保证），不存在该运行期分支。
//! - Java 通过 slot.getIteratorResult(类名) 取 Iterator；Rust 端 iterator
//!   节点直接返回 Value::Array。

use super::loop_condition::{LoopCondition, handle_future_list, run_sequential, submit_iteration};
use super::{Condition, ConditionBase};
use crate::enums::ConditionTypeEnum;
use crate::exception::{LFResult, LiteflowError};
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
pub struct IteratorCondition {
    base: ConditionBase,
    iterator_node: Arc<dyn Executable>,
    pub parallel: Option<usize>,
    thread_pool_executor_class: Option<String>,
    do_executor: Arc<dyn Executable>,
    break_item: Option<Arc<dyn Executable>>,
}

impl IteratorCondition {
    pub fn new(
        iterator_node: Arc<dyn Executable>,
        parallel: Option<usize>,
        do_executor: Arc<dyn Executable>,
        break_item: Option<Arc<dyn Executable>>,
    ) -> Self {
        Self {
            base: ConditionBase::default(),
            iterator_node,
            parallel,
            thread_pool_executor_class: None,
            do_executor,
            break_item,
        }
    }

    /// 执行 ITERATOR 条件主体。对应 Java: `IteratorCondition#executeCondition`。
    pub async fn execute_condition(&self, ctx: &Ctx, frame: &Frame) -> LFResult<Value> {
        <Self as Executable>::execute(self, ctx, frame).await
    }

    /// 返回条件类型。对应 Java: `IteratorCondition#getConditionType`。
    #[must_use]
    pub fn get_condition_type(&self) -> ConditionTypeEnum {
        ConditionTypeEnum::Iterator
    }

    /// 返回迭代数据节点。对应 Java: `IteratorCondition#getIteratorNode`。
    #[must_use]
    pub fn get_iterator_node(&self) -> &Arc<dyn Executable> {
        &self.iterator_node
    }

    /// 设置迭代数据节点。
    ///
    /// - `iterator_node`: 必须返回数组的可执行项。
    ///
    /// 对应 Java: `IteratorCondition#setIteratorNode`。
    pub fn set_iterator_node(&mut self, iterator_node: Arc<dyn Executable>) {
        self.iterator_node = iterator_node;
    }

    /// 返回条件类型的 Rust 惯用别名。
    pub fn condition_type(&self) -> ConditionTypeEnum {
        self.get_condition_type()
    }
}

#[async_trait]
impl Executable for IteratorCondition {
    async fn execute(&self, ctx: &Ctx, frame: &Frame) -> LFResult<Value> {
        super::execute_condition_with_lifecycle(self, ctx, frame, async {
            // 对应 Java IteratorCondition#executeCondition：先判断 isAccess，
            // 返回 false 则整个 ITERATOR 表达式不执行
            if !self.iterator_node.is_access(ctx, frame).await {
                return Ok(Value::Null);
            }
            let v = self.iterator_node.execute(ctx, frame).await?;
            let list = match &v {
                Value::Array(a) => a.clone(),
                other => {
                    return Err(LiteflowError::NodeTypeError {
                        node: self.iterator_node.id().to_string(),
                        expect: "array".into(),
                        actual: other.to_string(),
                    });
                }
            };

            if self.parallel.is_some() {
                let condition_key = format!("{:p}", self);
                let executor_service = ExecutorHelper::load_instance().build_executor_service(
                    self.thread_pool_executor_class
                        .as_deref()
                        .or(frame.condition_thread_pool()),
                    frame.chain_thread_pool(),
                    &condition_key,
                    &ctx.inner.chain_id,
                    ConditionTypeEnum::Iterator,
                )?;
                let mut set: JoinSet<LoopFutureObj> = JoinSet::new();
                for (i, obj) in list.iter().enumerate() {
                    if !submit_iteration(
                        self,
                        &mut set,
                        &self.do_executor,
                        self.break_item.as_ref(),
                        ctx,
                        frame,
                        i,
                        Some(obj.clone()),
                        &executor_service,
                    )
                    .await?
                    {
                        break;
                    }
                }
                return handle_future_list(set).await;
            }

            for (i, obj) in list.into_iter().enumerate() {
                if !run_sequential(
                    &self.do_executor,
                    self.break_item.as_ref(),
                    ctx,
                    frame,
                    i,
                    Some(obj),
                )
                .await?
                {
                    break;
                }
            }
            Ok(Value::Null)
        })
        .await
    }

    fn collect_node_ids(&self) -> Vec<String> {
        Condition::get_all_node_in_condition(self)
    }

    fn id(&self) -> &str {
        "ITERATOR"
    }
}

impl LoopCondition for IteratorCondition {
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
        self.parallel.is_some()
    }

    fn set_parallel(&mut self, parallel: bool) {
        if parallel {
            self.parallel.get_or_insert(0);
        } else {
            self.parallel = None;
        }
    }
}

impl Condition for IteratorCondition {
    fn condition_base(&self) -> &ConditionBase {
        &self.base
    }

    fn condition_base_mut(&mut self) -> &mut ConditionBase {
        &mut self.base
    }

    fn typed_executable_group(&self) -> HashMap<String, Vec<Arc<dyn Executable>>> {
        let mut groups = HashMap::from([
            (
                "ITERATOR_KEY".to_string(),
                vec![Arc::clone(&self.iterator_node)],
            ),
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
            "ITERATOR_KEY" if !executable_list.is_empty() => {
                self.iterator_node = Arc::clone(&executable_list[0]);
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
        IteratorCondition::condition_type(self)
    }
}
