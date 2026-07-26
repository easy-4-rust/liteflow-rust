//! 对应 Java: com.yomahub.liteflow.script.proxy.ScriptBeanProxy

use std::collections::{HashMap, HashSet};

use serde_json::Value;

use crate::exception::{LFResult, LiteflowError, ScriptBeanMethodInvokeException};
use crate::script::annotation::ScriptBean;

use super::ScriptMethodProxy;

/// 按包含/排除规则限制方法访问的脚本 Bean 代理。
#[derive(Clone)]
pub struct ScriptBeanProxy {
    bean_name: String,
    methods: HashMap<String, ScriptMethodProxy>,
}

impl ScriptBeanProxy {
    /// 根据注解元数据过滤可访问方法。
    ///
    /// 包含列表为空时默认允许全部候选方法；排除列表始终最后生效。
    #[must_use]
    pub fn new(
        bean_name: impl Into<String>,
        metadata: &ScriptBean,
        methods: impl IntoIterator<Item = ScriptMethodProxy>,
    ) -> Self {
        let includes = metadata
            .includes()
            .iter()
            .map(String::as_str)
            .collect::<HashSet<_>>();
        let excludes = metadata
            .excludes()
            .iter()
            .map(String::as_str)
            .collect::<HashSet<_>>();
        let methods = methods
            .into_iter()
            .filter(|method| {
                (includes.is_empty() || includes.contains(method.exposed_name()))
                    && !excludes.contains(method.exposed_name())
            })
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

    /// 调用允许暴露的方法；访问未声明或已排除的方法时返回专用异常。
    pub fn invoke(&self, method_name: &str, arguments: &[Value]) -> LFResult<Value> {
        self.methods
            .get(method_name)
            .ok_or_else(|| -> LiteflowError {
                ScriptBeanMethodInvokeException::new(
                    "",
                    format!(
                        "method[{method_name}] is not exposed by script bean[{}]",
                        self.bean_name
                    ),
                )
                .into()
            })?
            .invoke(arguments)
    }
}
