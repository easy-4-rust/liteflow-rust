//! 对应 Java: com.yomahub.liteflow.script.ScriptBeanManager

use std::sync::{Arc, OnceLock};

use dashmap::DashMap;
use serde_json::Value;

use crate::exception::{LFResult, ScriptBeanMethodInvokeException};

use super::proxy::ScriptBeanProxy;

fn script_beans() -> &'static DashMap<String, Arc<ScriptBeanProxy>> {
    static SCRIPT_BEANS: OnceLock<DashMap<String, Arc<ScriptBeanProxy>>> = OnceLock::new();
    SCRIPT_BEANS.get_or_init(DashMap::new)
}

/// 管理脚本可访问 Bean 的进程级注册表。
pub struct ScriptBeanManager;

impl ScriptBeanManager {
    /// 添加或覆盖脚本 Bean。对应 Java `addScriptBean`。
    pub fn add_script_bean(proxy: ScriptBeanProxy) {
        let proxy = proxy.get_proxy_script_bean();
        script_beans().insert(proxy.bean_name().to_string(), Arc::new(proxy));
    }

    /// 获取指定脚本 Bean。
    #[must_use]
    pub fn get_script_bean(bean_name: &str) -> Option<Arc<ScriptBeanProxy>> {
        script_beans().get(bean_name).map(|entry| entry.clone())
    }

    /// 返回当前 Bean 快照。对应 Java `getScriptBeanMap`，不暴露内部并发容器。
    #[must_use]
    pub fn get_script_bean_map() -> Vec<(String, Arc<ScriptBeanProxy>)> {
        let mut beans = script_beans()
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().clone()))
            .collect::<Vec<_>>();
        beans.sort_by(|left, right| left.0.cmp(&right.0));
        beans
    }

    /// 调用已注册 Bean 的方法。
    pub fn invoke(bean_name: &str, method_name: &str, arguments: &[Value]) -> LFResult<Value> {
        let bean = Self::get_script_bean(bean_name).ok_or_else(|| {
            ScriptBeanMethodInvokeException::new(format!(
                "script bean[{bean_name}] is not registered"
            ))
        })?;
        bean.invoke(method_name, arguments)
    }

    /// 移除指定 Bean，供热更新与测试隔离使用。
    pub fn remove_script_bean(bean_name: &str) {
        script_beans().remove(bean_name);
    }

    /// 清空脚本 Bean 注册表。
    pub fn clean() {
        script_beans().clear();
    }
}
