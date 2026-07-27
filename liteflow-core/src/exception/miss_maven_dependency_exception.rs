//! 对应 Java 类：com.yomahub.liteflow.exception.MissMavenDependencyException
//!
//! 缺少运行所需依赖（v2.16.0 新增）

use std::fmt;

use super::lite_flow_exception::LiteflowError;

/// Java 原实现使用的 Maven 依赖提示模板。
///
/// 注意：Java 调用的是 Hutool `StrUtil.format(String, Object...)`，该重载只替换
/// `{}`，不会替换这里的命名占位符；`${version}` 也会原样保留。
pub const TEMPLATE: &str = "miss maven dependency \n<dependency>\n    <groupId>{groupId}</groupId>\n    <artifactId>{artifactId}</artifactId>\n    <version>${version}</version>\n</dependency>";

/// 对应 MissMavenDependencyException：缺少运行所需依赖（v2.16.0 新增）
#[derive(Debug, Clone)]
pub struct MissMavenDependencyException {
    /// 异常信息
    pub message: String,
}

impl MissMavenDependencyException {
    /// 按 Java 构造器创建缺少 Maven 依赖的异常。
    ///
    /// 参数 `group_id`、`artifact_id` 分别对应 Java 的 `groupId`、`artifactId`。
    /// Java 当前 Hutool 调用不会替换命名占位符，因此 Rust 精确保留其实际输出。
    /// 对应 Java: `MissMavenDependencyException#MissMavenDependencyException`。
    pub fn new(group_id: impl Into<String>, artifact_id: impl Into<String>) -> Self {
        // 仍消费两个语义参数以保持构造签名一致；消息严格复现 Java 的实际格式化结果。
        let _ = (group_id.into(), artifact_id.into());
        Self {
            message: TEMPLATE.to_string(),
        }
    }

    /// 返回异常信息。
    ///
    /// # 返回
    /// 当前异常持有的原始消息。对应 Java: `MissMavenDependencyException#getMessage`。
    #[must_use]
    pub fn get_message(&self) -> &str {
        &self.message
    }

    /// 修改异常信息。
    ///
    /// 参数 `message` 对应 Java 同名参数。对应 Java:
    /// `MissMavenDependencyException#setMessage`。
    pub fn set_message(&mut self, message: impl Into<String>) {
        self.message = message.into();
    }
}

impl fmt::Display for MissMavenDependencyException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for MissMavenDependencyException {}

impl From<MissMavenDependencyException> for LiteflowError {
    fn from(e: MissMavenDependencyException) -> Self {
        LiteflowError::MissMavenDependency(e.message)
    }
}
