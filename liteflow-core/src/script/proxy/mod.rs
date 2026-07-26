//! 脚本 Bean 的受控代理。

pub mod script_bean_proxy;
pub mod script_method_proxy;

pub use script_bean_proxy::ScriptBeanProxy;
pub use script_method_proxy::{ScriptCallable, ScriptMethodProxy};
