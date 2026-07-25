//! 对应 DefaultContext + NodeComponent 的上下文访问方法集。

use crate::el::NodeRef;
use crate::slot::databus::Frame;
use crate::slot::slot::Slot;
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::any::Any;
use std::sync::atomic::Ordering;
use std::sync::Arc;

/// 传给组件的上下文
#[derive(Clone)]
pub struct CmpContext {
    pub inner: Arc<Slot>,
    pub node: NodeRef,
    pub frame: Frame,
}

impl CmpContext {
    pub fn request_id(&self) -> &str {
        &self.inner.request_id
    }
    pub fn chain_id(&self) -> &str {
        &self.inner.chain_id
    }
    pub fn node_id(&self) -> &str {
        &self.node.id
    }
    pub fn tag(&self) -> Option<&str> {
        self.node.tag.as_deref()
    }
    /// getCmpData(String.class)
    pub fn cmp_data(&self) -> Option<&str> {
        self.node.data.as_deref()
    }
    /// getCmpData 反序列化
    pub fn cmp_data_as<T: DeserializeOwned>(&self) -> Option<T> {
        self.node.data.as_deref().and_then(|s| serde_json::from_str(s).ok())
    }
    /// bindData
    pub fn bind_data(&self, key: &str) -> Option<&str> {
        self.node.bind.iter().find(|(k, _)| k == key).map(|(_, v)| v.as_str())
    }
    /// getRequestData()
    pub fn request_data<T: DeserializeOwned>(&self) -> Option<T> {
        self.inner.input.lock().ok().and_then(|v| serde_json::from_value(v.clone()).ok())
    }
    /// getContextBean / getFirstContextBean
    pub fn bean<T: Any + Send + Sync>(&self, name: &str) -> Option<Arc<T>> {
        self.inner.beans.get(name).and_then(|v| v.clone().downcast::<T>().ok())
    }
    pub fn set_data(&self, key: impl Into<String>, value: Value) {
        self.inner.data.insert(key.into(), value);
    }
    pub fn get_data(&self, key: &str) -> Option<Value> {
        self.inner.data.get(key).map(|v| v.clone())
    }
    pub fn get_data_as<T: DeserializeOwned>(&self, key: &str) -> Option<T> {
        self.inner.data.get(key).and_then(|v| serde_json::from_value(v.clone()).ok())
    }
    /// getLoopIndex()
    pub fn loop_index(&self) -> Option<usize> {
        self.frame.loop_index()
    }
    pub fn loop_index_at(&self, depth: usize) -> Option<usize> {
        self.frame.loop_index_at(depth)
    }
    /// getLoopObject()
    pub fn loop_object<T: DeserializeOwned>(&self) -> Option<T> {
        self.frame.loop_object().and_then(|v| serde_json::from_value(v.clone()).ok())
    }
    /// setIsEnd(true)
    pub fn end_chain(&self) {
        self.inner.ended.store(true, Ordering::Relaxed);
    }
}
