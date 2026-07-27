//! 组件执行步骤对象。
//!
//! 对应 Java: `com.yomahub.liteflow.flow.entity.CmpStep`。

use crate::core::NodeComponent;
use crate::enums::CmpStepTypeEnum;
use crate::flow::element::node::Node;
use serde_json::Value;
use std::fmt;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};

/// 保存一次组件执行或回滚的身份、耗时和异常信息。
///
/// Java 中的 `instance` 与 `refNode` 是执行期对象引用；Rust 同样保留其线程安全
/// `Arc`/克隆快照，用于诊断和对象级 API 对齐。真正的回滚顺序仍由 `Slot`
/// 内部队列控制，避免调用方修改响应步骤后破坏补偿语义。
/// 对应 Java: `com.yomahub.liteflow.flow.entity.CmpStep`。
#[derive(Clone)]
pub struct CmpStep {
    /// 同一节点在链路内的实例编号。
    pub node_instance_id: Option<String>,
    pub node_id: String,
    pub node_name: String,
    pub tag: Option<String>,
    pub step_type: CmpStepTypeEnum,
    /// Java `startTime`，记录组件开始执行的墙钟时间。
    pub start_time: SystemTime,
    /// Java `endTime`，组件尚未结束时为 `None`。
    pub end_time: Option<SystemTime>,
    pub time_spent: Option<Duration>,
    pub success: bool,
    pub exception: Option<String>,
    /// 执行该步骤的真实组件实例。
    pub instance: Option<Arc<dyn NodeComponent>>,
    pub rollback_time_spent: Option<Duration>,
    /// 当前执行 Node 的克隆快照。
    pub ref_node: Option<Node>,
    /// 组件在执行期间写入的自定义步骤数据。
    pub step_data: Option<Value>,
    pub thread_name: String,
    timer_started_at: Instant,
}

impl CmpStep {
    /// 创建尚未完成的组件步骤。
    ///
    /// - `node_id`: Java `nodeId`，组件节点标识。
    /// - `node_name`: Java `nodeName`，组件展示名称。
    /// - `step_type`: Java `stepType`，步骤类型。
    ///
    /// 对应 Java: `CmpStep#CmpStep(String, String, CmpStepTypeEnum)`。
    pub fn new(
        node_id: impl Into<String>,
        node_name: impl Into<String>,
        step_type: CmpStepTypeEnum,
    ) -> Self {
        Self {
            node_instance_id: None,
            node_id: node_id.into(),
            node_name: node_name.into(),
            tag: None,
            step_type,
            start_time: SystemTime::now(),
            end_time: None,
            time_spent: None,
            success: false,
            exception: None,
            instance: None,
            rollback_time_spent: None,
            ref_node: None,
            step_data: None,
            thread_name: std::thread::current()
                .name()
                .unwrap_or("unnamed")
                .to_string(),
            timer_started_at: Instant::now(),
        }
    }

    /// 完成正常执行计时并记录结果。
    ///
    /// 参数 `success` 与 `exception` 分别对应 Java 的成功状态与执行异常。
    pub fn finish(&mut self, success: bool, exception: Option<String>) {
        self.end_time = Some(SystemTime::now());
        self.time_spent = Some(self.timer_started_at.elapsed());
        self.success = success;
        self.exception = exception;
    }

    /// 完成回滚步骤计时。
    ///
    /// 对应 Java: `CmpStep#setRollbackTimeSpent`，回滚异常只记录在回滚步骤中，
    /// 不覆盖触发补偿的原始链路异常。
    pub fn finish_rollback(&mut self, success: bool, exception: Option<String>) {
        self.end_time = Some(SystemTime::now());
        self.rollback_time_spent = Some(self.timer_started_at.elapsed());
        self.success = success;
        self.exception = exception;
    }

    /// buildTimeSpent（毫秒）
    pub fn time_spent_ms(&self) -> u128 {
        self.time_spent.map(|d| d.as_millis()).unwrap_or(0)
    }

    /// 回滚耗时，单位毫秒。
    pub fn rollback_time_spent_ms(&self) -> u128 {
        self.rollback_time_spent
            .map(|duration| duration.as_millis())
            .unwrap_or(0)
    }

    /// 返回节点实例编号。对应 Java: `CmpStep#getNodeInstanceId`。
    #[must_use]
    pub fn get_node_instance_id(&self) -> Option<&str> {
        self.node_instance_id.as_deref()
    }

    /// 设置节点实例编号。对应 Java: `CmpStep#setNodeInstanceId`。
    pub fn set_node_instance_id(&mut self, node_instance_id: impl Into<String>) {
        self.node_instance_id = Some(node_instance_id.into());
    }

    /// 返回节点 ID。对应 Java: `CmpStep#getNodeId`。
    #[must_use]
    pub fn get_node_id(&self) -> &str {
        &self.node_id
    }

    /// 设置节点 ID。对应 Java: `CmpStep#setNodeId`。
    pub fn set_node_id(&mut self, node_id: impl Into<String>) {
        self.node_id = node_id.into();
    }

    /// 返回步骤类型。对应 Java: `CmpStep#getStepType`。
    #[must_use]
    pub fn get_step_type(&self) -> CmpStepTypeEnum {
        self.step_type
    }

    /// 设置步骤类型。对应 Java: `CmpStep#setStepType`。
    pub fn set_step_type(&mut self, step_type: CmpStepTypeEnum) {
        self.step_type = step_type;
    }

    /// 返回节点名称。对应 Java: `CmpStep#getNodeName`。
    #[must_use]
    pub fn get_node_name(&self) -> &str {
        &self.node_name
    }

    /// 设置节点名称。对应 Java: `CmpStep#setNodeName`。
    pub fn set_node_name(&mut self, node_name: impl Into<String>) {
        self.node_name = node_name.into();
    }

    /// 返回执行耗时（毫秒）。对应 Java: `CmpStep#getTimeSpent`。
    #[must_use]
    pub fn get_time_spent(&self) -> Option<u128> {
        self.time_spent.map(|duration| duration.as_millis())
    }

    /// 设置执行耗时，单位毫秒。
    ///
    /// 参数 `time_spent` 对应 Java `Long timeSpent`。对应 Java:
    /// `CmpStep#setTimeSpent`。
    pub fn set_time_spent(&mut self, time_spent: u64) {
        self.time_spent = Some(Duration::from_millis(time_spent));
    }

    /// 返回步骤是否执行成功。对应 Java: `CmpStep#isSuccess`。
    #[must_use]
    pub fn is_success(&self) -> bool {
        self.success
    }

    /// 设置步骤成功状态。对应 Java: `CmpStep#setSuccess`。
    pub fn set_success(&mut self, success: bool) {
        self.success = success;
    }

    /// 返回异常文本。对应 Java: `CmpStep#getException`。
    #[must_use]
    pub fn get_exception(&self) -> Option<&str> {
        self.exception.as_deref()
    }

    /// 设置异常文本。对应 Java: `CmpStep#setException`。
    pub fn set_exception(&mut self, exception: Option<String>) {
        self.exception = exception;
    }

    /// 返回执行该步骤的组件实例。
    ///
    /// 对应 Java: `CmpStep#getInstance`。
    #[must_use]
    pub fn get_instance(&self) -> Option<&Arc<dyn NodeComponent>> {
        self.instance.as_ref()
    }

    /// 设置执行该步骤的组件实例。
    ///
    /// 参数 `instance` 对应 Java `NodeComponent instance`。对应 Java:
    /// `CmpStep#setInstance`。
    pub fn set_instance(&mut self, instance: Arc<dyn NodeComponent>) {
        self.instance = Some(instance);
    }

    /// 返回回滚耗时（毫秒）。对应 Java: `CmpStep#getRollbackTimeSpent`。
    #[must_use]
    pub fn get_rollback_time_spent(&self) -> Option<u128> {
        self.rollback_time_spent
            .map(|duration| duration.as_millis())
    }

    /// 设置回滚耗时，单位毫秒。
    ///
    /// 对应 Java: `CmpStep#setRollbackTimeSpent`。
    pub fn set_rollback_time_spent(&mut self, rollback_time_spent: u64) {
        self.rollback_time_spent = Some(Duration::from_millis(rollback_time_spent));
    }

    /// 返回当前执行 Node 的快照。对应 Java: `CmpStep#getRefNode`。
    #[must_use]
    pub fn get_ref_node(&self) -> Option<&Node> {
        self.ref_node.as_ref()
    }

    /// 设置当前执行 Node，并同步节点实例编号。
    ///
    /// 对应 Java: `CmpStep#setRefNode`。
    pub fn set_ref_node(&mut self, ref_node: Node) {
        self.node_instance_id = ref_node.get_node_instance_id().map(ToOwned::to_owned);
        self.ref_node = Some(ref_node);
    }

    /// 构建不含耗时的步骤文本。
    ///
    /// 节点名为空时返回 `nodeId`，否则返回 `nodeId[nodeName]`。
    /// 对应 Java: `CmpStep#buildString`。
    #[must_use]
    pub fn build_string(&self) -> String {
        if self.node_name.trim().is_empty() {
            self.node_id.clone()
        } else {
            format!("{}[{}]", self.node_id, self.node_name)
        }
    }

    /// 构建包含节点实例编号的步骤文本。
    ///
    /// 对应 Java: `CmpStep#buildStringWithInstanceId`。
    #[must_use]
    pub fn build_string_with_instance_id(&self) -> String {
        format!(
            "{}[{}]",
            self.node_id,
            self.node_instance_id.as_deref().unwrap_or_default()
        )
    }

    /// 构建包含执行耗时的步骤文本。
    ///
    /// 对应 Java: `CmpStep#buildStringWithTime`。
    #[must_use]
    pub fn build_string_with_time(&self) -> String {
        match self.get_time_spent() {
            Some(time_spent) if self.node_name.trim().is_empty() => {
                format!("{}<{}>", self.node_id, time_spent)
            }
            Some(time_spent) => {
                format!("{}[{}]<{}>", self.node_id, self.node_name, time_spent)
            }
            None => self.build_string(),
        }
    }

    /// 构建包含回滚耗时的步骤文本。
    ///
    /// 对应 Java: `CmpStep#buildRollbackStringWithTime`。
    #[must_use]
    pub fn build_rollback_string_with_time(&self) -> String {
        match self.get_rollback_time_spent() {
            Some(time_spent) if self.node_name.trim().is_empty() => {
                format!("{}<{}>", self.node_id, time_spent)
            }
            Some(time_spent) => {
                format!("{}[{}]<{}>", self.node_id, self.node_name, time_spent)
            }
            None => self.build_string(),
        }
    }

    /// 按节点 ID 判断两个步骤是否相等。
    ///
    /// Java `equals` 明确忽略实例编号、耗时与成功状态。对应 Java:
    /// `CmpStep#equals`。
    #[must_use]
    pub fn equals(&self, other: &Self) -> bool {
        self.node_id == other.node_id
    }

    /// 返回节点标签。对应 Java: `CmpStep#getTag`。
    #[must_use]
    pub fn get_tag(&self) -> Option<&str> {
        self.tag.as_deref()
    }

    /// 设置节点标签。对应 Java: `CmpStep#setTag`。
    pub fn set_tag(&mut self, tag: impl Into<String>) {
        self.tag = Some(tag.into());
    }

    /// 返回组件开始执行的墙钟时间。对应 Java: `CmpStep#getStartTime`。
    #[must_use]
    pub fn get_start_time(&self) -> SystemTime {
        self.start_time
    }

    /// 设置组件开始执行的墙钟时间。
    ///
    /// 对应 Java: `CmpStep#setStartTime`。
    pub fn set_start_time(&mut self, start_time: SystemTime) {
        self.start_time = start_time;
        self.timer_started_at = Instant::now();
    }

    /// 返回组件结束时间。对应 Java: `CmpStep#getEndTime`。
    #[must_use]
    pub fn get_end_time(&self) -> Option<SystemTime> {
        self.end_time
    }

    /// 设置组件结束时间。对应 Java: `CmpStep#setEndTime`。
    pub fn set_end_time(&mut self, end_time: SystemTime) {
        self.end_time = Some(end_time);
    }

    /// 返回自定义步骤数据。对应 Java: `CmpStep#getStepData`。
    #[must_use]
    pub fn get_step_data(&self) -> Option<&Value> {
        self.step_data.as_ref()
    }

    /// 设置自定义步骤数据。
    ///
    /// Java `Object` 映射为 `serde_json::Value`。对应 Java:
    /// `CmpStep#setStepData`。
    pub fn set_step_data(&mut self, step_data: Value) {
        self.step_data = Some(step_data);
    }

    /// 返回执行线程名称。对应 Java: `CmpStep#getThreadName`。
    #[must_use]
    pub fn get_thread_name(&self) -> &str {
        &self.thread_name
    }

    /// 设置执行线程名称。对应 Java: `CmpStep#setThreadName`。
    pub fn set_thread_name(&mut self, thread_name: impl Into<String>) {
        self.thread_name = thread_name.into();
    }
}

impl PartialEq for CmpStep {
    fn eq(&self, other: &Self) -> bool {
        self.equals(other)
    }
}

impl Eq for CmpStep {}

impl fmt::Debug for CmpStep {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CmpStep")
            .field("node_instance_id", &self.node_instance_id)
            .field("node_id", &self.node_id)
            .field("node_name", &self.node_name)
            .field("tag", &self.tag)
            .field("step_type", &self.step_type)
            .field("start_time", &self.start_time)
            .field("end_time", &self.end_time)
            .field("time_spent", &self.time_spent)
            .field("success", &self.success)
            .field("exception", &self.exception)
            .field("rollback_time_spent", &self.rollback_time_spent)
            .field("step_data", &self.step_data)
            .field("thread_name", &self.thread_name)
            .finish_non_exhaustive()
    }
}
