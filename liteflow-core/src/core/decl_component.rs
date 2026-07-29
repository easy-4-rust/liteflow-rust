//! 声明式组件的 Rust 动态分派契约。

use crate::enums::NodeTypeEnum;
use crate::exception::LiteflowError;
use crate::slot::CmpContext;
use async_trait::async_trait;
use serde_json::Value;

/// 将一个组件暴露的多个具名方法统一到对象安全的异步分派入口。
///
/// 这是 Rust 专用边界，承接 Java `DeclComponentProxy`、`@LiteflowCmpDefine`
/// 与 `@LiteflowMethod` 的组合职责；EL 以 `cmpId.methodName` 引用方法。
#[async_trait]
pub trait DeclComponent: Send + Sync + 'static {
    /// 按方法名执行声明式组件逻辑。
    ///
    /// 参数 `method` 是 Java LiteFlowMethodEnum 对应的方法名；`ctx` 是真实组件
    /// 执行上下文。成功时返回方法结果，失败时返回 LiteflowError。
    async fn call(&self, method: &str, ctx: &CmpContext) -> Result<Value, LiteflowError>;

    /// 调用需要当前执行异常的声明式方法。
    ///
    /// 默认兼容不声明异常参数的方法；过程宏生成的 `ON_ERROR` 方法会覆盖该入口并
    /// 收到真实错误。对应 Java: `DeclComponentProxy#loadMethodParameter` 对
    /// `NodeComponent#onError(Exception)` 参数的转发。
    async fn call_with_error(
        &self,
        method: &str,
        ctx: &CmpContext,
        _error: &LiteflowError,
    ) -> Result<Value, LiteflowError> {
        self.call(method, ctx).await
    }

    /// 判断是否存在指定声明式方法。
    ///
    /// 参数 `method` 是待检查方法名；返回 true 表示代理可以分派。
    /// 手写旧实现无法枚举方法，默认保持兼容；`DeclComponentProxy` 会执行精确检查。
    fn has_method(&self, _method: &str) -> bool {
        true
    }

    /// 返回声明式方法映射的节点类型。
    ///
    /// 参数 `method` 是待查询方法名；无显式类型时返回 `None`。
    fn method_node_type(&self, _method: &str) -> Option<NodeTypeEnum> {
        None
    }

    /// 返回声明式方法的节点显示名。
    ///
    /// 参数 `method` 是待查询方法名；无自定义名称时返回 `None`。
    fn method_name<'a>(&'a self, _method: &str) -> Option<&'a str> {
        None
    }

    /// 返回声明式主方法的重试次数。
    ///
    /// 参数 `method` 是待查询方法名；未声明重试时返回 0。
    fn method_retry_count(&self, _method: &str) -> usize {
        0
    }

    /// 判断声明式方法是否应针对指定错误重试。
    ///
    /// 参数 `method`、`error` 分别是方法名和本次错误；返回 true 时进入下一次重试。
    fn is_method_retry_for(&self, _method: &str, _error: &LiteflowError) -> bool {
        false
    }

    /// 返回指定 Java 生命周期角色对应的真实业务方法名。
    ///
    /// 参数 `liteflow_method` 对应 `@LiteflowMethod#value`；返回值是宏生成静态
    /// 分派表中的 Rust 方法名。旧手写实现仍按 Java 标准方法名兼容。
    /// 对应 Java: `DeclComponentProxy.AopInvocationHandler#invoke` 的方法角色查找。
    fn method_for_lifecycle(
        &self,
        liteflow_method: crate::enums::LiteFlowMethodEnum,
    ) -> Option<&str> {
        let method = liteflow_method.get_method_name();
        self.has_method(method).then_some(method)
    }
}
