//! 对应各 RulePlugin 的规则内容获取契约。

use async_trait::async_trait;

use crate::exception::LFResult;
use crate::rule_plugin::RuleFormat;

/// 规则源接口。
///
/// 对应 Java 各规则插件 Parser 的 `getContent` 与格式识别公共语义。
#[async_trait]
pub trait RuleSource: Send + Sync + 'static {
    /// 拉取规则文本与版本指纹，用于变更检测。
    async fn fetch(&self) -> LFResult<(String, String)>;

    /// 返回规则格式。
    fn format(&self) -> RuleFormat;

    /// 返回用于诊断和日志的规则源名称。
    fn name(&self) -> &str;
}
