//! 对应 Slot：一次链路执行的共享状态。

use crate::flow::entity::cmp_step::CmpStep;
use dashmap::DashMap;
use serde_json::Value;
use std::any::Any;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

pub struct Slot {
    pub request_id: String,
    pub chain_id: String,
    /// contextBeanMap
    pub beans: DashMap<String, Arc<dyn Any + Send + Sync>>,
    /// requestData
    pub input: Mutex<Value>,
    /// 链路内共享数据
    pub data: DashMap<String, Value>,
    /// executeSteps
    pub steps: Mutex<Vec<CmpStep>>,
    /// slot.exception
    pub exception: Mutex<Option<String>>,
    /// isEnd
    pub ended: AtomicBool,
}

impl Slot {
    pub fn new(request_id: String, chain_id: impl Into<String>, input: Value) -> Self {
        Self {
            request_id,
            chain_id: chain_id.into(),
            beans: DashMap::new(),
            input: Mutex::new(input),
            data: DashMap::new(),
            steps: Mutex::new(Vec::new()),
            exception: Mutex::new(None),
            ended: AtomicBool::new(false),
        }
    }
}
