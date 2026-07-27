//! 对应 Java: com.yomahub.liteflow.script.ScriptExecuteWrap

use std::sync::Arc;

use serde_json::Value;

use crate::core::NodeComponent;
use crate::slot::CmpContext;

/// 传递给脚本执行器的节点执行快照。
///
/// 该对象把组件执行时的 Slot、当前 Chain、节点、循环和配置数据固定为一次调用
/// 快照，避免脚本引擎跨越异步边界后再读取可变线程局部状态。对应 Java:
/// `com.yomahub.liteflow.script.ScriptExecuteWrap`。
#[derive(Clone, Default)]
pub struct ScriptExecuteWrap {
    slot_index: Option<usize>,
    curr_chain_id: String,
    node_id: String,
    tag: Option<String>,
    cmp_data: Option<String>,
    loop_index: Option<usize>,
    loop_object: Option<Value>,
    component: Option<Arc<dyn NodeComponent>>,
}

impl ScriptExecuteWrap {
    /// 从真实组件执行上下文创建脚本参数快照。
    #[must_use]
    pub fn from_context(context: &CmpContext) -> Self {
        Self {
            slot_index: context.slot_index(),
            curr_chain_id: context.curr_chain_id().to_string(),
            node_id: context.node_id().to_string(),
            tag: context.tag().map(str::to_string),
            cmp_data: context.cmp_data().map(str::to_string),
            loop_index: context.loop_index(),
            loop_object: context.frame.loop_object().cloned(),
            component: None,
        }
    }

    /// 返回执行槽位索引。
    #[must_use]
    pub fn slot_index(&self) -> Option<usize> {
        self.slot_index
    }

    /// 返回执行槽位索引。
    ///
    /// 返回 `None` 表示 Slot 已释放或该快照尚未绑定 Slot。对应 Java:
    /// `ScriptExecuteWrap#getSlotIndex`。
    #[must_use]
    pub fn get_slot_index(&self) -> Option<usize> {
        self.slot_index()
    }

    /// 设置执行槽位索引。
    pub fn set_slot_index(&mut self, slot_index: Option<usize>) {
        self.slot_index = slot_index;
    }
    /// 返回当前链路 id。
    #[must_use]
    pub fn curr_chain_id(&self) -> &str {
        &self.curr_chain_id
    }

    /// 返回当前正在执行的 Chain ID。
    ///
    /// 返回值对应 Java `currChainId` 字段。对应 Java:
    /// `ScriptExecuteWrap#getCurrChainId`。
    #[must_use]
    pub fn get_curr_chain_id(&self) -> &str {
        self.curr_chain_id()
    }

    /// 设置当前链路 id。
    pub fn set_curr_chain_id(&mut self, curr_chain_id: impl Into<String>) {
        self.curr_chain_id = curr_chain_id.into();
    }
    /// 兼容 Java 旧版 `currChainName`。
    #[deprecated(note = "请使用 curr_chain_id")]
    #[must_use]
    pub fn curr_chain_name(&self) -> &str {
        self.curr_chain_id()
    }

    /// 返回兼容旧版本的当前 Chain 名称。
    ///
    /// Java 2.16 已以 `currChainId` 为准，此方法保留旧 API 的同值语义。对应 Java:
    /// `ScriptExecuteWrap#getCurrChainName`。
    #[deprecated(note = "请使用 get_curr_chain_id")]
    #[must_use]
    pub fn get_curr_chain_name(&self) -> &str {
        self.get_curr_chain_id()
    }

    /// 兼容 Java 旧版 `setCurrChainName`。
    #[deprecated(note = "请使用 set_curr_chain_id")]
    pub fn set_curr_chain_name(&mut self, curr_chain_name: impl Into<String>) {
        self.set_curr_chain_id(curr_chain_name);
    }
    /// 返回节点 id。
    #[must_use]
    pub fn node_id(&self) -> &str {
        &self.node_id
    }

    /// 返回脚本节点 ID。
    ///
    /// 返回值用于从执行器编译缓存中定位脚本。对应 Java:
    /// `ScriptExecuteWrap#getNodeId`。
    #[must_use]
    pub fn get_node_id(&self) -> &str {
        self.node_id()
    }

    /// 设置节点 id。
    pub fn set_node_id(&mut self, node_id: impl Into<String>) {
        self.node_id = node_id.into();
    }
    /// 返回节点标签。
    #[must_use]
    pub fn tag(&self) -> Option<&str> {
        self.tag.as_deref()
    }

    /// 返回脚本节点标签。
    ///
    /// 节点未声明标签时返回 `None`。对应 Java: `ScriptExecuteWrap#getTag`。
    #[must_use]
    pub fn get_tag(&self) -> Option<&str> {
        self.tag()
    }

    /// 设置节点标签。
    pub fn set_tag(&mut self, tag: Option<String>) {
        self.tag = tag;
    }
    /// 返回节点配置数据。
    #[must_use]
    pub fn cmp_data(&self) -> Option<&str> {
        self.cmp_data.as_deref()
    }

    /// 返回节点配置数据快照。
    ///
    /// Rust 保留规则文件中的原始 JSON/文本，具体脚本引擎按需反序列化。对应 Java:
    /// `ScriptExecuteWrap#getCmpData`。
    #[must_use]
    pub fn get_cmp_data(&self) -> Option<&str> {
        self.cmp_data()
    }

    /// 设置节点配置数据。
    pub fn set_cmp_data(&mut self, cmp_data: Option<String>) {
        self.cmp_data = cmp_data;
    }
    /// 返回循环下标。
    #[must_use]
    pub fn loop_index(&self) -> Option<usize> {
        self.loop_index
    }

    /// 返回当前循环下标。
    ///
    /// 不在循环节点内执行时返回 `None`。对应 Java:
    /// `ScriptExecuteWrap#getLoopIndex`。
    #[must_use]
    pub fn get_loop_index(&self) -> Option<usize> {
        self.loop_index()
    }

    /// 设置循环下标。
    pub fn set_loop_index(&mut self, loop_index: Option<usize>) {
        self.loop_index = loop_index;
    }
    /// 返回循环对象。
    #[must_use]
    pub fn loop_object(&self) -> Option<&Value> {
        self.loop_object.as_ref()
    }

    /// 返回当前循环对象。
    ///
    /// Java `Object` 映射为 `serde_json::Value`；非迭代场景返回 `None`。对应 Java:
    /// `ScriptExecuteWrap#getLoopObject`。
    #[must_use]
    pub fn get_loop_object(&self) -> Option<&Value> {
        self.loop_object()
    }

    /// 设置循环对象。
    pub fn set_loop_object(&mut self, loop_object: Option<Value>) {
        self.loop_object = loop_object;
    }
    /// 返回当前脚本组件。
    #[must_use]
    pub fn component(&self) -> Option<Arc<dyn NodeComponent>> {
        self.component.clone()
    }

    /// 返回当前脚本组件。
    ///
    /// 通过克隆 `Arc` 保留 Java 共享组件引用语义。对应 Java:
    /// `ScriptExecuteWrap#getCmp`。
    #[must_use]
    pub fn get_cmp(&self) -> Option<Arc<dyn NodeComponent>> {
        self.component()
    }

    /// 设置当前脚本组件。
    pub fn set_component(&mut self, component: Option<Arc<dyn NodeComponent>>) {
        self.component = component;
    }

    /// 设置当前脚本组件。
    ///
    /// 参数 `cmp` 对应 Java 同名参数；`None` 用于清除绑定。对应 Java:
    /// `ScriptExecuteWrap#setCmp`。
    pub fn set_cmp(&mut self, cmp: Option<Arc<dyn NodeComponent>>) {
        self.set_component(cmp);
    }
}
