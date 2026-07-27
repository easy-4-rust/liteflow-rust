//! 节点执行路径帧。

use serde_json::Value;

/// 对应 Java Node 的 loopIndexTL/loopObjectTL 与 Slot.conditionStack。
#[derive(Debug, Clone, Default)]
pub struct Frame {
    /// `(loopIndex, loopObject)` 栈。
    pub loops: Vec<(usize, Option<Value>)>,
    /// Condition 级 bind 键值栈。
    pub binds: Vec<(String, String)>,
    /// 当前 Chain 指定的执行器构建器名称。
    chain_thread_pool: Option<String>,
    /// 当前 Condition 指定的执行器构建器名称。
    condition_thread_pool: Option<String>,
    /// 当前 SWITCH 条件允许跳转的目标节点 ID。
    switch_target_list: Vec<String>,
}

impl Frame {
    /// 创建根执行帧。
    #[must_use]
    pub fn root() -> Self {
        Self::default()
    }

    /// 压入循环下标和循环对象。
    #[must_use]
    pub fn push(&self, index: usize, object: Option<Value>) -> Self {
        let mut frame = self.clone();
        frame.loops.push((index, object));
        frame
    }

    /// 压入 Condition 级绑定数据。
    #[must_use]
    pub fn push_bind(&self, pairs: &[(String, String)]) -> Self {
        if pairs.is_empty() {
            return self.clone();
        }
        let mut frame = self.clone();
        frame.binds.extend(pairs.iter().cloned());
        frame
    }

    /// 写入当前 Chain 的执行器构建器名称。
    ///
    /// 对应 Java `Chain#setThreadPoolExecutorClass` 在执行期间由
    /// `ExecutorConditionBuilder` 读取的链级配置。
    #[must_use]
    pub fn with_chain_thread_pool(&self, thread_pool: Option<&str>) -> Self {
        let mut frame = self.clone();
        frame.chain_thread_pool = thread_pool.map(ToOwned::to_owned);
        frame
    }

    /// 写入当前 Condition 的执行器构建器名称。
    ///
    /// 对应 Java `LoopCondition#setThreadPoolExecutorClass`。
    #[must_use]
    pub fn with_condition_thread_pool(&self, thread_pool: Option<&str>) -> Self {
        let mut frame = self.clone();
        frame.condition_thread_pool = thread_pool.map(ToOwned::to_owned);
        frame
    }

    /// 写入当前 SWITCH 条件的目标节点 ID 列表。
    ///
    /// 参数 `target_list` 对应 Java `SwitchCondition#getTargetList` 中可执行对象的
    /// ID 投影；返回携带该条件上下文的新执行帧，供 `NodeSwitchComponent` 在路由
    /// 计算期间读取。
    #[must_use]
    pub fn with_switch_target_list(&self, target_list: &[String]) -> Self {
        let mut frame = self.clone();
        frame.switch_target_list = target_list.to_vec();
        frame
    }

    /// 返回当前 Chain 的执行器构建器名称。
    #[must_use]
    pub fn chain_thread_pool(&self) -> Option<&str> {
        self.chain_thread_pool.as_deref()
    }

    /// 返回当前 Condition 的执行器构建器名称。
    #[must_use]
    pub fn condition_thread_pool(&self) -> Option<&str> {
        self.condition_thread_pool.as_deref()
    }

    /// 返回当前 SWITCH 条件允许跳转的目标节点 ID。
    ///
    /// 返回值对应 Java `NodeSwitchComponent#getTargetList`。
    #[must_use]
    pub fn switch_target_list(&self) -> &[String] {
        &self.switch_target_list
    }

    /// 从栈顶向下查找绑定数据。
    #[must_use]
    pub fn find_bind(&self, key: &str) -> Option<&str> {
        self.binds
            .iter()
            .rev()
            .find(|(existing, _)| existing == key)
            .map(|(_, value)| value.as_str())
    }

    /// 返回最内层循环下标。
    #[must_use]
    pub fn loop_index(&self) -> Option<usize> {
        self.loops.last().map(|(index, _)| *index)
    }

    /// 返回最内层循环对象。
    #[must_use]
    pub fn loop_object(&self) -> Option<&Value> {
        self.loops.last().and_then(|(_, object)| object.as_ref())
    }

    /// 按深度返回循环下标，0 表示最内层。
    #[must_use]
    pub fn loop_index_at(&self, depth: usize) -> Option<usize> {
        self.loops
            .len()
            .checked_sub(depth + 1)
            .and_then(|index| self.loops.get(index))
            .map(|(index, _)| *index)
    }
}
