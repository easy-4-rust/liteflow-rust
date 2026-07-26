//! Java `LiteFlowMethodBean` 的 Rust 映射。

use crate::enums::LiteFlowMethodEnum;

/// 保存声明式组件方法的名称及其 LiteFlow 生命周期角色。
///
/// Java 版本保存 `java.lang.reflect.Method`；Rust 不使用运行期反射，实际调用由
/// `DeclComponent` 的编译期静态分派完成，因此用 `LiteFlowMethodEnum` 保存同等的
/// 可验证方法角色。
///
/// 对应 Java: `com.yomahub.liteflow.core.proxy.LiteFlowMethodBean`。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiteFlowMethodBean {
    method_name: String,
    method: LiteFlowMethodEnum,
}

impl LiteFlowMethodBean {
    /// 创建方法元数据。
    ///
    /// `method_name` 对应 Java 反射方法名，`method` 对应其
    /// `@LiteflowMethod#value`。对应 Java: `LiteFlowMethodBean#LiteFlowMethodBean`。
    #[must_use]
    pub fn new(method_name: impl Into<String>, method: LiteFlowMethodEnum) -> Self {
        Self {
            method_name: method_name.into(),
            method,
        }
    }

    /// 返回声明式方法名。对应 Java: `LiteFlowMethodBean#getMethodName`。
    #[must_use]
    pub fn method_name(&self) -> &str {
        &self.method_name
    }

    /// 修改声明式方法名。对应 Java: `LiteFlowMethodBean#setMethodName`。
    pub fn set_method_name(&mut self, method_name: impl Into<String>) {
        self.method_name = method_name.into();
    }

    /// 返回方法的 LiteFlow 生命周期角色。
    ///
    /// 对应 Java: `LiteFlowMethodBean#getMethod`，Rust 以枚举替代反射对象。
    #[must_use]
    pub fn method(&self) -> LiteFlowMethodEnum {
        self.method
    }

    /// 修改方法的 LiteFlow 生命周期角色。
    ///
    /// 对应 Java: `LiteFlowMethodBean#setMethod`。
    pub fn set_method(&mut self, method: LiteFlowMethodEnum) {
        self.method = method;
    }
}
