//! 对应 Java: com.yomahub.liteflow.script.proxy.ScriptBeanProxy

use std::collections::{HashMap, HashSet};

use serde_json::Value;

use crate::exception::{LFResult, LiteflowError, ScriptBeanMethodInvokeException};

use super::ScriptMethodProxy;

/// 按包含/排除规则限制方法访问的脚本 Bean 代理。
#[derive(Clone)]
pub struct ScriptBeanProxy {
    bean_name: String,
    methods: HashMap<String, ScriptMethodProxy>,
}

impl ScriptBeanProxy {
    /// 根据调用方提供的包含/排除方法清单过滤可访问方法。
    ///
    /// 包含列表为空时默认允许全部候选方法；排除列表始终最后生效。
    #[must_use]
    pub fn new(
        bean_name: impl Into<String>,
        include_method_names: &[&str],
        exclude_method_names: &[&str],
        methods: impl IntoIterator<Item = ScriptMethodProxy>,
    ) -> Self {
        let includes = include_method_names.iter().copied().collect::<HashSet<_>>();
        let excludes = exclude_method_names.iter().copied().collect::<HashSet<_>>();
        let methods = methods
            .into_iter()
            .filter(|method| {
                (includes.is_empty() || includes.contains(method.exposed_name()))
                    && !excludes.contains(method.exposed_name())
            })
            // Java 在这里为每个允许的方法创建 ByteBuddy 代理；Rust 完成同一
            // 白名单决策后，把静态 callable 代理转移到最终 Bean 代理中。
            .map(ScriptMethodProxy::get_proxy_script_method)
            .map(|method| (method.exposed_name().to_string(), method))
            .collect();
        Self {
            bean_name: bean_name.into(),
            methods,
        }
    }

    /// 返回脚本侧 Bean 名称。
    #[must_use]
    pub fn bean_name(&self) -> &str {
        &self.bean_name
    }

    /// 返回排序后的可调用方法名，便于诊断与生成工具提示。
    #[must_use]
    pub fn method_names(&self) -> Vec<String> {
        let mut names = self.methods.keys().cloned().collect::<Vec<_>>();
        names.sort();
        names
    }

    /// 生成完成包含/排除过滤后的脚本 Bean 代理。
    ///
    /// Java 返回 ByteBuddy 创建的新对象；Rust 对象在 `new` 中已经完成同一方法
    /// 白名单构建并持有真实 callable，因此所有权转移后即可注册到脚本引擎。
    /// 返回值仍会拒绝未声明或被排除的方法。
    ///
    /// 对应 Java: `ScriptBeanProxy#getProxyScriptBean`。
    #[must_use]
    pub fn get_proxy_script_bean(self) -> Self {
        self
    }

    /// 调用允许暴露的方法；访问未声明或已排除的方法时返回专用异常。
    pub fn invoke(&self, method_name: &str, arguments: &[Value]) -> LFResult<Value> {
        self.methods
            .get(method_name)
            .ok_or_else(|| -> LiteflowError {
                ScriptBeanMethodInvokeException::new(format!(
                    "method[{method_name}] is not exposed by script bean[{}]",
                    self.bean_name
                ))
                .into()
            })?
            .invoke(arguments)
    }
}
