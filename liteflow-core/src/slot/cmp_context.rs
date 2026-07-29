//! LiteFlow 组件执行期的 Rust 类型安全访问视图。
//!
//! 这是 Rust 异步执行模型的适配对象；Java 对等的业务上下文对象位于
//! `default_context.rs`，两者不再混放在同一文件。

use crate::el::NodeRef;
use crate::slot::slot::Slot;
use crate::slot::{DataBus, Frame};
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::any::Any;
use std::sync::Arc;
use std::sync::atomic::Ordering;

/// 传给组件的类型安全执行上下文。
///
/// `node` 与 `frame` 由每次 Node 调用显式创建并随异步任务传递，替代 Java
/// `NodeComponent#setRefNode/removeRefNode` 的 ThreadLocal 栈；嵌套调用通过
/// 上下文克隆隔离，无需易泄漏的手工清理。
pub struct CmpContext {
    pub inner: Arc<Slot>,
    pub node: NodeRef,
    pub frame: Frame,
}

impl Clone for CmpContext {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            node: self.node.clone(),
            frame: self.frame.clone_for_component(),
        }
    }
}

impl CmpContext {
    /// 返回当前执行 Slot 在 DataBus 中的索引。
    ///
    /// Slot 执行结束并释放后返回 None。对应 Java:
    /// `NodeComponent#getSlotIndex`。
    pub fn slot_index(&self) -> Option<usize> {
        DataBus::get_slot_index(&self.inner)
    }

    /// 返回本次流程请求 ID。对应 Java: `NodeComponent#getRequestId`。
    #[must_use]
    pub fn request_id(&self) -> &str {
        &self.inner.request_id
    }

    /// 返回 Slot 保存的主 Chain ID。对应 Java: `NodeComponent#getChainId`。
    #[must_use]
    pub fn chain_id(&self) -> &str {
        &self.inner.chain_id
    }
    /// 返回当前正在执行的 Chain ID；根链未显式写入 Frame 时回退到 Slot 主链。
    ///
    /// 对应 Java: `NodeComponent#getCurrChainId`。
    pub fn curr_chain_id(&self) -> &str {
        self.frame.current_chain_id().unwrap_or(self.chain_id())
    }
    /// 返回当前组件节点 ID。对应 Java: `NodeComponent#getNodeId`。
    #[must_use]
    pub fn node_id(&self) -> &str {
        &self.node.id
    }

    /// 返回当前节点标签；未配置时返回 `None`。
    ///
    /// 对应 Java: `NodeComponent#getTag`。
    #[must_use]
    pub fn tag(&self) -> Option<&str> {
        self.node.tag.as_deref()
    }
    /// getCmpData(String.class)
    pub fn cmp_data(&self) -> Option<&str> {
        self.frame
            .chain_cmp_data()
            .or_else(|| self.node.data.as_deref())
            .filter(|data| !data.trim().is_empty())
    }
    /// getCmpData 反序列化
    pub fn cmp_data_as<T: DeserializeOwned>(&self) -> Option<T> {
        self.cmp_data()
            .and_then(|data| serde_json::from_str(data).ok())
    }
    /// bindData（2.16 语义：先查 Node 级 bind，找不到再从
    /// Condition 栈顶向下查找，正确处理 chain.bind / THEN(...).bind 场景）
    pub fn bind_data(&self, key: &str) -> Option<&str> {
        if let Some(v) = self
            .node
            .bind
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
            .filter(|value| !value.trim().is_empty())
        {
            return Some(v);
        }
        self.frame
            .find_bind(key)
            .filter(|value| !value.trim().is_empty())
    }

    /// 返回当前 SWITCH 表达式可选择的目标节点 ID。
    ///
    /// Java 组件通过 `NodeSwitchComponent#getTargetList` 从 Slot 的当前 Condition
    /// 获取该列表；Rust 将不可变条件上下文随 `Frame` 传入组件，避免线程局部变量。
    /// 不在 SWITCH 路由节点中调用时返回空列表。
    #[must_use]
    pub fn switch_target_list(&self) -> Vec<String> {
        self.frame.switch_target_list().to_vec()
    }
    /// getRequestData()
    pub fn request_data<T: DeserializeOwned>(&self) -> Option<T> {
        self.inner
            .input
            .lock()
            .ok()
            .and_then(|v| serde_json::from_value(v.clone()).ok())
    }
    /// getContextBean / getFirstContextBean
    pub fn bean<T: Any + Send + Sync>(&self, name: &str) -> Option<Arc<T>> {
        self.inner
            .beans
            .get(name)
            .and_then(|v| v.clone().downcast::<T>().ok())
    }
    /// 向本次执行的共享数据区写入 JSON 值。
    ///
    /// 参数 `key`、`value` 分别是数据键和值；同名键会被覆盖。这是 Rust 组件
    /// 上下文的便利入口，事实来源仍为 Java Slot 的共享状态。
    pub fn set_data(&self, key: impl Into<String>, value: Value) {
        self.inner.data.insert(key.into(), value);
    }

    /// 返回共享数据区中指定 JSON 值的快照。
    ///
    /// 参数 `key` 是数据键；不存在时返回 `None`。
    #[must_use]
    pub fn get_data(&self, key: &str) -> Option<Value> {
        self.inner.data.get(key).map(|v| v.clone())
    }

    /// 将共享数据区中的值反序列化为目标类型。
    ///
    /// 参数 `key` 是数据键；不存在或 serde 转换失败时返回 `None`。
    #[must_use]
    pub fn get_data_as<T: DeserializeOwned>(&self, key: &str) -> Option<T> {
        self.inner
            .data
            .get(key)
            .and_then(|v| serde_json::from_value(v.clone()).ok())
    }

    /// 设置当前组件步骤的自定义数据。
    ///
    /// Java `NodeComponent#setStepData(Object)` 最终写入 `Node.stepDataTL`；
    /// Rust 使用任务隔离的 `Frame` 锁保存 serde 值。对应 Java:
    /// `NodeComponent#setStepData`。
    pub fn set_step_data(&self, step_data: Value) {
        self.frame
            .set_node_step_data(self.node.id.clone(), step_data);
    }

    /// 返回当前组件步骤自定义数据。对应 Java: `NodeComponent#getStepData`。
    #[must_use]
    pub fn get_step_data(&self) -> Option<Value> {
        self.frame.get_node_step_data(&self.node.id)
    }
    /// getLoopIndex()
    pub fn loop_index(&self) -> Option<usize> {
        self.frame.loop_index()
    }
    /// 返回指定嵌套深度的循环下标。
    ///
    /// 参数 `depth` 从当前循环向外计数，0 表示最内层；不存在时返回 `None`。
    /// 承接 Java LoopCondition 的多层循环 ThreadLocal 查询语义。
    #[must_use]
    pub fn loop_index_at(&self, depth: usize) -> Option<usize> {
        self.frame.loop_index_at(depth)
    }
    /// getLoopObject()
    pub fn loop_object<T: DeserializeOwned>(&self) -> Option<T> {
        self.frame
            .loop_object()
            .and_then(|v| serde_json::from_value(v.clone()).ok())
    }
    /// setIsEnd(true)
    pub fn end_chain(&self) {
        self.inner.ended.store(true, Ordering::Relaxed);
    }
    /// conversationId（2.15+）
    pub fn conversation_id(&self) -> Option<&str> {
        self.inner.conversation_id.as_deref()
    }
    /// 发布执行事件（对应 FlowEventPublisher.publish(getSlot(), event)，2.15+）
    pub fn publish_event(&self, event: &crate::flow::flow_event::FlowEvent) {
        crate::flow::flow_event_publisher::FlowEventPublisher::publish_ctx(&self.inner, event);
    }
    /// Slot attachment 访问（2.15+）
    pub fn set_attachment<T: Any + Send + Sync>(&self, key: impl Into<String>, value: T) {
        self.inner.set_attachment(key, value);
    }
    /// 按类型读取 Slot attachment。
    ///
    /// 参数 `key` 是 attachment 键；不存在或类型不匹配时返回 `None`。
    /// 对应 Java: `Slot#getAttachment`。
    #[must_use]
    pub fn get_attachment<T: Any + Send + Sync>(&self, key: &str) -> Option<Arc<T>> {
        self.inner.get_attachment(key)
    }

    /// 返回 Slot 是否存在指定 attachment。
    ///
    /// 参数 `key` 是待查询键。对应 Java: `Slot#hasAttachment`。
    #[must_use]
    pub fn has_attachment(&self, key: &str) -> bool {
        self.inner.has_attachment(key)
    }

    /// 移除 Slot 中指定 attachment。
    ///
    /// 参数 `key` 是待删除键；不存在时静默完成。对应 Java:
    /// `Slot#removeAttachment`。
    pub fn remove_attachment(&self, key: &str) {
        self.inner.remove_attachment(key);
    }
}
