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

    /// 生成只允许调用已注册脚本方法的代理对象。
    ///
    /// Java 通过 ByteBuddy 创建一个新运行期类型；Rust 的 `ScriptMethodProxy`
    /// 本身已经持有类型擦除后的真实 callable，因此所有权转移后直接成为最终代理，
    /// 不需要反射或运行期代码生成。
    ///
    /// 返回值保留真实业务闭包及暴露名称。对应 Java:
    /// `ScriptMethodProxy#getProxyScriptMethod`。
    #[must_use]
    pub fn get_proxy_script_method(self) -> Self {
        self
    }

    /// 调用被代理的真实方法。
    pub fn invoke(&self, arguments: &[Value]) -> LFResult<Value> {
        (self.callable)(arguments)
    }
}
