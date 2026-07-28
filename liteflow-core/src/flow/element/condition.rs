//! Condition 抽象基座及 `flow.element.condition` 子包入口。

pub mod abstract_condition;
pub mod and_or_condition;
pub mod bind_wrapper_condition;
pub mod boolean_condition_type_enum;
pub mod catch_condition;
pub mod chain_bind_wrapper_condition;
pub mod condition_key;
pub mod finally_condition;
pub mod for_condition;
pub mod if_condition;
pub mod ignore_error_condition;
pub mod iterator_condition;
pub mod loop_condition;
pub mod not_condition;
pub mod pre_condition;
pub mod retry_condition;
pub mod switch_condition;
pub mod then_condition;
pub mod timeout_condition;
pub mod when_condition;
pub mod while_condition;

pub use boolean_condition_type_enum::BooleanConditionTypeEnum;

use crate::enums::ConditionTypeEnum;
use crate::enums::ExecuteableTypeEnum;
use crate::exception::{LFResult, LiteflowError};
use crate::flow::element::Executable;
use crate::slot::{Ctx, Frame};
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use std::fmt;
use std::future::Future;
use std::sync::Arc;

/// Condition 抽象类的公共对象状态。
///
/// 该类型是 Rust 为复用 Java 抽象类字段而使用的内部伴随状态，不对应新的
/// Java 对象。具体 Condition 必须真实持有它，不能通过全局表或空默认值模拟。
/// 对应 Java: `com.yomahub.liteflow.flow.element.Condition` 的
/// `id/tag/currChainId/bindDataMap` 字段。
#[doc(hidden)]
#[derive(Clone, Default)]
pub struct ConditionBase {
    id: Option<String>,
    tag: Option<String>,
    curr_chain_id: Option<String>,
    bind_data: Vec<(String, String)>,
    executable_group: HashMap<String, Vec<Arc<dyn Executable>>>,
}

impl ConditionBase {
    /// 使用当前 Chain id 创建公共状态。
    #[must_use]
    pub(crate) fn with_curr_chain_id(curr_chain_id: impl Into<String>) -> Self {
        Self {
            curr_chain_id: Some(curr_chain_id.into()),
            ..Self::default()
        }
    }

    fn put_bind_data(&mut self, key: String, value: String) {
        self.bind_data
            .retain(|(existing_key, _)| existing_key != &key);
        self.bind_data.push((key, value));
    }

    /// 返回当前 Condition 保存的绑定数据。
    ///
    /// 返回切片保持插入顺序，并确保同名键只保留最后一次赋值；对应 Java
    /// `Condition#getBindDataMap` 的构建期读取语义。
    pub(crate) fn bind_data(&self) -> &[(String, String)] {
        &self.bind_data
    }
}

impl fmt::Debug for ConditionBase {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ConditionBase")
            .field("id", &self.id)
            .field("tag", &self.tag)
            .field("curr_chain_id", &self.curr_chain_id)
            .field("bind_data", &self.bind_data)
            .field(
                "executable_group_keys",
                &self.executable_group.keys().collect::<Vec<_>>(),
            )
            .finish()
    }
}

/// 所有流程条件的统一抽象。
///
/// Java 抽象类的公共元数据由每个具体对象内嵌的 `ConditionBase` 承载；
/// `Executable::execute` 继续作为 Rust 对象安全异步入口。可执行对象分组仍由各
/// 具体 Condition 的强类型字段承载，下一阶段会统一暴露分组访问接口。
///
/// 对应 Java: `com.yomahub.liteflow.flow.element.Condition`。
#[async_trait]
pub trait Condition: Executable {
    /// 返回当前对象真实持有的公共状态。
    fn condition_base(&self) -> &ConditionBase;

    /// 返回当前对象真实持有的可变公共状态。
    fn condition_base_mut(&mut self) -> &mut ConditionBase;

    /// 返回由具体 Condition 强类型字段承载的可执行分组。
    ///
    /// 该内部扩展点使公共 Java Map API 与具体执行字段保持单一事实来源。
    #[doc(hidden)]
    fn typed_executable_group(&self) -> HashMap<String, Vec<Arc<dyn Executable>>> {
        HashMap::new()
    }

    /// 尝试用分组列表更新具体 Condition 的强类型执行字段。
    ///
    /// 返回 `true` 表示该 key 已由具体类型接管；未知 key 由公共状态保存。
    #[doc(hidden)]
    fn replace_typed_executable_group(
        &mut self,
        _group_key: &str,
        _executable_list: Vec<Arc<dyn Executable>>,
    ) -> bool {
        false
    }

    /// 执行具体 Condition 主体。
    ///
    /// 参数 `ctx/frame` 对应 Java 的 `slotIndex` 所定位的 Slot 与线程局部状态。
    /// 对应 Java: `Condition#executeCondition`。
    async fn execute_condition(&self, ctx: &Ctx, frame: &Frame) -> LFResult<Value> {
        <Self as Executable>::execute(self, ctx, frame).await
    }

    /// 返回统一可执行对象类型。对应 Java: `Condition#getExecuteType`。
    #[must_use]
    fn get_execute_type(&self) -> ExecuteableTypeEnum {
        ExecuteableTypeEnum::Condition
    }

    /// 返回条件类型。对应 Java: `Condition#getConditionType`。
    fn condition_type(&self) -> ConditionTypeEnum;

    /// 返回条件类型的 Java 命名入口。对应 Java: `Condition#getConditionType`。
    #[must_use]
    fn get_condition_type(&self) -> ConditionTypeEnum {
        self.condition_type()
    }

    /// 返回显式 id；未提供时按 Java 规则生成 `condition-{type}`。
    ///
    /// 对应 Java: `Condition#getId`。
    fn condition_id(&self) -> String {
        self.get_id()
    }

    /// 返回显式 id；空值按 Java 规则生成 `condition-{type}`。
    ///
    /// 对应 Java: `Condition#getId`。
    #[must_use]
    fn get_id(&self) -> String {
        self.condition_base()
            .id
            .as_deref()
            .filter(|id| !id.trim().is_empty())
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| format!("condition-{}", self.condition_type().get_name()))
    }

    /// 设置条件 id。参数 `id` 对应 Java 同名参数。
    ///
    /// 对应 Java: `Condition#setId`。
    fn set_id(&mut self, id: impl Into<String>)
    where
        Self: Sized,
    {
        self.condition_base_mut().id = Some(id.into());
    }

    /// 返回条件标签。对应 Java: `Condition#getTag`。
    fn condition_tag(&self) -> Option<&str> {
        self.get_tag()
    }

    /// 返回条件标签。对应 Java: `Condition#getTag`。
    #[must_use]
    fn get_tag(&self) -> Option<&str> {
        self.condition_base().tag.as_deref()
    }

    /// 设置条件标签。参数 `tag` 对应 Java 同名参数。
    ///
    /// 对应 Java: `Condition#setTag`。
    fn set_tag(&mut self, tag: impl Into<String>)
    where
        Self: Sized,
    {
        self.condition_base_mut().tag = Some(tag.into());
    }

    /// 返回当前 Chain id 的旧名称兼容入口。
    ///
    /// 对应 Java: `Condition#getCurrChainName`（已废弃）。
    #[must_use]
    fn get_curr_chain_name(&self) -> Option<&str> {
        self.get_curr_chain_id()
    }

    /// 返回当前 Chain id。对应 Java: `Condition#getCurrChainId`。
    #[must_use]
    fn get_curr_chain_id(&self) -> Option<&str> {
        self.condition_base().curr_chain_id.as_deref()
    }

    /// 设置当前 Chain id。参数 `curr_chain_id` 对应 Java 同名参数。
    ///
    /// 对应 Java: `Condition#setCurrChainId`。
    fn set_curr_chain_id(&mut self, curr_chain_id: impl Into<String>)
    where
        Self: Sized,
    {
        self.condition_base_mut().curr_chain_id = Some(curr_chain_id.into());
    }

    /// 写入 Condition 级 bind 数据。
    ///
    /// 相同 key 后写覆盖先写；顺序保留用于稳定诊断。对应 Java:
    /// `Condition#putBindData`。
    fn put_bind_data(&mut self, key: impl Into<String>, value: impl Into<String>)
    where
        Self: Sized,
    {
        self.condition_base_mut()
            .put_bind_data(key.into(), value.into());
    }

    /// 返回指定 key 的 bind 数据。对应 Java: `Condition#getBindData`。
    #[must_use]
    fn get_bind_data(&self, key: &str) -> Option<&str> {
        self.condition_base()
            .bind_data
            .iter()
            .rev()
            .find(|(existing_key, _)| existing_key == key)
            .map(|(_, value)| value.as_str())
    }

    /// 判断是否存在指定 key 的 bind 数据。
    ///
    /// 对应 Java: `Condition#hasBindData`。
    #[must_use]
    fn has_bind_data(&self, key: &str) -> bool {
        self.get_bind_data(key).is_some()
    }

    /// 返回指定分组的可执行对象快照。
    ///
    /// 空分组返回空列表；`group_key` 对应 Java 同名参数。对应 Java:
    /// `Condition#getExecutableList(String)`。
    #[must_use]
    fn get_executable_list(&self, group_key: &str) -> Vec<Arc<dyn Executable>> {
        self.get_executable_group()
            .remove(group_key)
            .unwrap_or_default()
    }

    /// 返回指定分组的首个可执行对象。
    ///
    /// 空分组返回 `None`，对应 Java 的 `null`。对应 Java:
    /// `Condition#getExecutableOne`。
    #[must_use]
    fn get_executable_one(&self, group_key: &str) -> Option<Arc<dyn Executable>> {
        self.get_executable_list(group_key).into_iter().next()
    }

    /// 返回 Condition 中递归展开的全部 Node id。
    ///
    /// 遍历会沿 executableGroup 的当前迭代顺序进入嵌套 Condition 和 Chain；
    /// 子列表顺序与重复项均保留，不做排序或去重。对应 Java:
    /// `Condition#getAllNodeInCondition`。
    #[must_use]
    fn get_all_node_in_condition(&self) -> Vec<String> {
        self.get_executable_group()
            .into_values()
            .flatten()
            .flat_map(|executable| executable.collect_node_ids())
            .collect()
    }

    /// 替换默认分组的可执行对象。
    ///
    /// 参数 `executable_list` 对应 Java 同名参数。对应 Java:
    /// `Condition#setExecutableList`。
    fn set_executable_list(&mut self, executable_list: Vec<Arc<dyn Executable>>)
    where
        Self: Sized,
    {
        self.replace_executable_group("DEFAULT_KEY", executable_list);
    }

    /// 向默认分组添加一个可执行对象。
    ///
    /// 参数 `executable` 对应 Java 同名参数。对应 Java:
    /// `Condition#addExecutable(Executable)`。
    fn add_executable(&mut self, executable: Arc<dyn Executable>)
    where
        Self: Sized,
    {
        self.add_executable_to_group("DEFAULT_KEY", executable);
    }

    /// 向指定分组添加一个可执行对象。
    ///
    /// 对应 Java: `Condition#addExecutable(String, Executable)`。
    fn add_executable_to_group(&mut self, group_key: &str, executable: Arc<dyn Executable>)
    where
        Self: Sized,
    {
        let mut executable_list = self.get_executable_list(group_key);
        executable_list.push(executable);
        self.replace_executable_group(group_key, executable_list);
    }

    /// 返回全部可执行对象分组快照。
    ///
    /// 强类型字段覆盖公共状态中的同名 key，确保执行路径和 Java Map API 读取同一
    /// 事实来源。对应 Java: `Condition#getExecutableGroup`。
    #[must_use]
    fn get_executable_group(&self) -> HashMap<String, Vec<Arc<dyn Executable>>> {
        let mut executable_group = self.condition_base().executable_group.clone();
        executable_group.extend(self.typed_executable_group());
        executable_group
    }

    /// 更新一个可执行对象分组。
    fn replace_executable_group(
        &mut self,
        group_key: &str,
        executable_list: Vec<Arc<dyn Executable>>,
    ) where
        Self: Sized,
    {
        if self.replace_typed_executable_group(group_key, executable_list.clone()) {
            self.condition_base_mut().executable_group.remove(group_key);
        } else {
            self.condition_base_mut()
                .executable_group
                .insert(group_key.to_string(), executable_list);
        }
    }
}

/// 以 Java `Condition#execute` 的统一生命周期执行具体条件主体。
///
/// 执行前把当前 Condition 快照压入任务局部栈；`ChainEnd` 只作为主动结束信号
/// 上抛，其他错误同时写入 Slot；无论成功或失败都在返回前弹栈。`Frame` 克隆时
/// 会复制栈快照，对齐 Java `TransmittableThreadLocal` 的子任务继承语义。
pub(crate) async fn execute_condition_with_lifecycle<C, F>(
    condition: &C,
    ctx: &Ctx,
    frame: &Frame,
    body: F,
) -> LFResult<Value>
where
    C: Condition + Clone + 'static,
    F: Future<Output = LFResult<Value>>,
{
    frame.push_condition(Arc::new(condition.clone()));
    let result = body.await;
    if let Err(error) = &result {
        if !matches!(error, LiteflowError::ChainEnd(_)) {
            ctx.set_exception(&error.to_string());
        }
    }
    frame.pop_condition();
    result
}

/// 期望布尔结果的元素返回其他类型时报错。
///
/// 对应 Java `IfTypeErrorException` / `SwitchTypeErrorException` 的类型校验。
pub fn expect_bool(name: &str, value: &Value) -> LFResult<bool> {
    match value {
        Value::Bool(result) => Ok(*result),
        other => Err(LiteflowError::NodeTypeError {
            node: name.to_string(),
            expect: "boolean".into(),
            actual: other.to_string(),
        }),
    }
}

/// 校验 IF / SWITCH 的目标不是 PRE 或 FINALLY。
pub fn check_not_pre_finally(target: &dyn Executable, name: &str) -> LFResult<()> {
    if target.is_pre_or_finally() {
        Err(LiteflowError::TargetCannotBePreOrFinally(name.to_string()))
    } else {
        Ok(())
    }
}
