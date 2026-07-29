//! 对应 Java 类：com.yomahub.liteflow.flow.element.condition.ForCondition
//!
//! 计数循环、DO/BREAK 与 PARALLEL 布尔并行开关。
//!
//! 差异说明：
//! - Java 在 forNode 为空时抛 NoForNodeException；Rust 端由 builder 保证
//!   `for_node` 与 `fixed_count` 必有其一，不存在该运行期分支。
//! - Java 通过 slot.getForResult(类名) 取循环次数；Rust 端 for 节点直接
//!   返回 Value::Number。

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
pub struct ForCondition {
    base: ConditionBase,
    for_node: Option<Arc<dyn Executable>>,
    fixed_count: Option<usize>,
    pub parallel: bool,
    thread_pool_executor_class: Option<String>,
    do_executor: Arc<dyn Executable>,
    break_item: Option<Arc<dyn Executable>>,
}

impl ForCondition {
    /// 创建由 FOR 节点动态计算次数的循环 Condition。
    ///
    /// 参数 `for_node` 产生循环次数，`parallel` 是 Java 布尔并行开关，`do_executor`
    /// 是循环体，`break_item` 是可选 BREAK 条件。对应 Java:
    /// `ForCondition` 由 EL Builder 完成字段装配后的执行状态。
    #[must_use]
    pub fn new(
        for_node: Arc<dyn Executable>,
        parallel: bool,
        do_executor: Arc<dyn Executable>,
        break_item: Option<Arc<dyn Executable>>,
    ) -> Self {
        Self {
            base: ConditionBase::default(),
            for_node: Some(for_node),
            fixed_count: None,
            parallel,
            thread_pool_executor_class: None,
            do_executor,
            break_item,
        }
    }

    /// 创建固定次数循环，对应 EL Builder 的 `FOR(Integer)` 重载。
    pub fn with_count(
        count: usize,
        parallel: bool,
        do_executor: Arc<dyn Executable>,
        break_item: Option<Arc<dyn Executable>>,
    ) -> Self {
        Self {
            base: ConditionBase::default(),
            for_node: None,
            fixed_count: Some(count),
            parallel,
            thread_pool_executor_class: None,
            do_executor,
            break_item,
        }
    }

    /// 执行 FOR 条件主体。对应 Java: `ForCondition#executeCondition`。
    pub async fn execute_condition(&self, ctx: &Ctx, frame: &Frame) -> LFResult<Value> {
        <Self as Executable>::execute(self, ctx, frame).await
    }

    /// 返回条件类型。对应 Java: `ForCondition#getConditionType`。
    #[must_use]
    pub fn get_condition_type(&self) -> ConditionTypeEnum {
        ConditionTypeEnum::For
    }

    /// 返回动态循环次数节点。
    ///
    /// 固定次数 `FOR(Integer)` 没有节点，返回 `None`。
    /// 对应 Java: `ForCondition#getForNode`。
    #[must_use]
    pub fn get_for_node(&self) -> Option<&Arc<dyn Executable>> {
        self.for_node.as_ref()
    }

    /// 设置动态循环次数节点，并清除固定次数模式。
    ///
    /// 对应 Java: `ForCondition#setForNode`。
    pub fn set_for_node(&mut self, for_node: Arc<dyn Executable>) {
        self.for_node = Some(for_node);
        self.fixed_count = None;
    }

    /// 返回条件类型的 Rust 惯用别名。
    pub fn condition_type(&self) -> ConditionTypeEnum {
        self.get_condition_type()
    }
}

#[async_trait]
impl Executable for ForCondition {
    async fn execute(&self, ctx: &Ctx, frame: &Frame) -> LFResult<Value> {
        super::execute_condition_with_lifecycle(self, ctx, frame, async {
            let count = if let Some(count) = self.fixed_count {
                count
            } else {
                let for_node = self
                    .for_node
                    .as_ref()
                    .expect("ForCondition 必须包含节点或固定次数");
                // 对应 Java ForCondition#executeCondition：先判断 isAccess，
                // 返回 false 则整个 FOR 表达式不执行
                if !for_node.is_access(ctx, frame).await {
                    return Ok(Value::Null);
                }
                let v = for_node.execute(ctx, frame).await?;
                match &v {
                    // Java NodeForComponent#getItemResultMetaValue 返回 Integer：
                    // 非负整数作为循环次数，负数按 `i < forCount` 语义执行零次。
                    Value::Number(number) if number.as_u64().is_some() => {
                        number.as_u64().expect("已验证为非负整数") as usize
                    }
                    Value::Number(number) if number.as_i64().is_some() => 0,
                    other => {
                        return Err(LiteflowError::NodeTypeError {
                            node: for_node.id().to_string(),
                            expect: "integer".into(),
                            actual: other.to_string(),
                        });
                    }
                }
            };

            if self.parallel {
                let condition_key = format!("{:p}", self);
                let executor_service = ExecutorHelper::load_instance().build_executor_service(
                    self.thread_pool_executor_class
                        .as_deref()
                        .or(frame.condition_thread_pool()),
                    frame.chain_thread_pool(),
                    &condition_key,
                    &ctx.inner.chain_id,
                    ConditionTypeEnum::For,
                )?;
                let mut set: JoinSet<LoopFutureObj> = JoinSet::new();
                for i in 0..count {
                    let should_continue = submit_iteration(
                        self,
                        &mut set,
                        &self.do_executor,
                        self.break_item.as_ref(),
                        ctx,
                        frame,
                        i,
                        None,
                        &executor_service,
                    )
                    .await?;
                    if should_continue {
                        continue;
                    }
                    break;
                }
                return handle_future_list(set).await;
            }

            for i in 0..count {
                let should_continue = run_sequential(
                    &self.do_executor,
                    self.break_item.as_ref(),
                    ctx,
                    frame,
                    i,
                    None,
                )
                .await?;
                if should_continue {
                    continue;
                }
                break;
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
        "FOR"
    }
}

impl LoopCondition for ForCondition {
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

impl Condition for ForCondition {
    fn condition_base(&self) -> &ConditionBase {
        &self.base
    }

    fn condition_base_mut(&mut self) -> &mut ConditionBase {
        &mut self.base
    }

    fn typed_executable_group(&self) -> HashMap<String, Vec<Arc<dyn Executable>>> {
        let mut groups =
            HashMap::from([("DO_KEY".to_string(), vec![Arc::clone(&self.do_executor)])]);
        if let Some(for_node) = &self.for_node {
            groups.insert("FOR_KEY".to_string(), vec![Arc::clone(for_node)]);
        }
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
            "FOR_KEY" => {
                self.for_node = executable_list.into_iter().next();
                self.fixed_count = None;
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
        ForCondition::condition_type(self)
    }
}
