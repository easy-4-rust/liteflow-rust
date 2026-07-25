use std::collections::HashMap;

use crate::flow::element::{ConditionKey, Executable};

/// 节点执行器辅助类
///
/// 用于获取实例缓存和缓存实例
#[derive(Default)]
pub struct NodeExecutorHelper {
    /// 实例缓存，key 为条件key，value 为条件实例
    instance_cache: HashMap<ConditionKey, Box<dyn Executable>>,
}

impl NodeExecutorHelper {
    /// 获取实例缓存
    ///
    /// 如果实例缓存中没有，则返回 None
    ///
    /// 否则，返回实例缓存中的实例
    pub fn get_instance(&self, key: &ConditionKey) -> Option<&dyn Executable> {
        self.instance_cache.get(key).map(|x| x.as_ref())
    }

    /// 缓存实例
    ///
    /// 如果实例缓存中没有，则缓存实例
    ///
    /// 否则，不做任何操作
    pub fn cache_instance(&mut self, executable: Box<dyn Executable>) {
        let key = executable.condition_key();
        self.instance_cache.entry(key).or_insert(executable);
    }
}
