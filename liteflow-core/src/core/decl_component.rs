//! 对应 core.proxy.DeclComponentProxy + annotation.LiteflowMethod：
//! 声明式组件——一个组件暴露多个具名方法，EL 中以 `cmpId.methodName` 引用。

use crate::enums::NodeTypeEnum;
use crate::exception::LiteflowError;
use crate::slot::CmpContext;
use async_trait::async_trait;
use serde_json::Value;

/// 声明式组件（对应 @LiteflowCmpDefine 类 + @LiteflowMethod 方法）
#[async_trait]
pub trait DeclComponent: Send + Sync + 'static {
    /// 按方法名调用（对应 LiteFlowMethodEnum 分派）
    async fn call(&self, method: &str, ctx: &CmpContext) -> Result<Value, LiteflowError>;

    /// 判断是否存在指定声明式方法。
    ///
    /// 手写旧实现无法枚举方法，默认保持兼容；`DeclComponentProxy` 会执行精确检查。
    fn has_method(&self, _method: &str) -> bool {
        true
    }

    /// 返回声明式方法映射的节点类型。
    fn method_node_type(&self, _method: &str) -> Option<NodeTypeEnum> {
        None
    }

    /// 返回声明式方法的节点显示名。
    fn method_name<'a>(&'a self, _method: &str) -> Option<&'a str> {
        None
    }

    /// 返回声明式主方法的重试次数。
    fn method_retry_count(&self, _method: &str) -> usize {
        0
    }

    /// 判断声明式方法是否应针对指定错误重试。
    fn is_method_retry_for(&self, _method: &str, _error: &LiteflowError) -> bool {
        false
    }
}
