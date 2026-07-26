//! 对应 Slot：一次链路执行的共享状态。

use crate::core::NodeComponent;
use crate::el::NodeRef;
use crate::flow::entity::cmp_step::CmpStep;
use crate::slot::Frame;
use dashmap::DashMap;
use serde_json::Value;
use std::any::Any;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

pub struct Slot {
    pub request_id: String,
    pub chain_id: String,
    /// conversationId（2.15+：业务会话标识，ReAct Agent 连续对话场景）
    pub conversation_id: Option<String>,
    /// contextBeanMap
    pub beans: DashMap<String, Arc<dyn Any + Send + Sync>>,
    /// requestData
    pub input: Mutex<Value>,
    /// 链路内共享数据
    pub data: DashMap<String, Value>,
    /// executeSteps
    pub steps: Mutex<Vec<CmpStep>>,
    /// rollbackSteps（对应 Java Slot#getRollbackSteps）
    pub rollback_steps: Mutex<Vec<CmpStep>>,
    /// 已进入执行阶段且声明支持回滚的组件。
    ///
    /// 元组依次为 NodeInstanceId、组件实例、NodeRef 与分支 Frame。这里不保存
    /// 含 `Arc<Slot>` 的完整 CmpContext，避免 Slot 自身形成强引用环。发生普通异常
    /// 或超时时由 FlowExecutor 逆序消费；同一 NodeInstanceId 因重试产生多条记录
    /// 时只回滚一次，对齐 Java `NodeComponent#doRollback` 的去重语义。
    pub(crate) rollback_items: Mutex<Vec<(String, Arc<dyn NodeComponent>, NodeRef, Frame)>>,
    /// slot.exception
    pub exception: Mutex<Option<String>>,
    /// isEnd
    pub ended: AtomicBool,
    /// attachment（2.15+：Slot.setAttachment/getAttachment/hasAttachment/removeAttachment）
    pub attachments: DashMap<String, Arc<dyn Any + Send + Sync>>,
}

impl Slot {
    pub fn new(request_id: String, chain_id: impl Into<String>, input: Value) -> Self {
        Self {
            request_id,
            chain_id: chain_id.into(),
            conversation_id: None,
            beans: DashMap::new(),
            input: Mutex::new(input),
            data: DashMap::new(),
            steps: Mutex::new(Vec::new()),
            rollback_steps: Mutex::new(Vec::new()),
            rollback_items: Mutex::new(Vec::new()),
            exception: Mutex::new(None),
            ended: AtomicBool::new(false),
            attachments: DashMap::new(),
        }
    }

    /// setAttachment(key, value)
    pub fn set_attachment<T: Any + Send + Sync>(&self, key: impl Into<String>, value: T) {
        self.attachments.insert(key.into(), Arc::new(value));
    }
    /// getAttachment(key)
    pub fn get_attachment<T: Any + Send + Sync>(&self, key: &str) -> Option<Arc<T>> {
        self.attachments
            .get(key)
            .and_then(|v| v.clone().downcast::<T>().ok())
    }
    /// hasAttachment(key)
    pub fn has_attachment(&self, key: &str) -> bool {
        self.attachments.contains_key(key)
    }
    /// removeAttachment(key)
    pub fn remove_attachment(&self, key: &str) {
        self.attachments.remove(key);
    }
}
