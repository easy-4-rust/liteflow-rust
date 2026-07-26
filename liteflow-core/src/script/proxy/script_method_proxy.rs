//! 对应 Java: com.yomahub.liteflow.script.proxy.ScriptMethodProxy

use std::sync::Arc;

use serde_json::Value;

use crate::exception::LFResult;

/// 可由脚本调用的类型擦除函数。
pub type ScriptCallable = Arc<dyn Fn(&[Value]) -> LFResult<Value> + Send + Sync>;

/// 只暴露显式注册方法的脚本代理。
#[derive(Clone)]
pub struct ScriptMethodProxy {
    exposed_name: String,
    callable: ScriptCallable,
}

impl ScriptMethodProxy {
    /// 创建方法代理。
    ///
    /// 参数 `exposed_name` 对应 `@ScriptMethod(value)`；`callable` 保留真实业务逻辑。
    #[must_use]
    pub fn new(exposed_name: impl Into<String>, callable: ScriptCallable) -> Self {
        Self {
            exposed_name: exposed_name.into(),
            callable,
        }
    }

    /// 返回脚本侧方法名。
    #[must_use]
    pub fn exposed_name(&self) -> &str {
        &self.exposed_name
    }

    /// 调用被代理的真实方法。
    pub fn invoke(&self, arguments: &[Value]) -> LFResult<Value> {
        (self.callable)(arguments)
    }
}
