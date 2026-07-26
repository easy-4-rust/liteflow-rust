//! 规则解析器工厂接口。

use crate::exception::LFResult;
use crate::parser::base::FlowParser;

/// 创建 JSON、XML、YML 三类 EL 解析器的统一工厂协议。
///
/// Rust 以 `Box<dyn FlowParser>` 表达 Java 基类返回值，并用 `Result`
/// 显式承接反射/注册失败。
///
/// 对应 Java: `com.yomahub.liteflow.parser.factory.FlowParserFactory`。
pub trait FlowParserFactory: Send + Sync {
    /// 创建 JSON EL 解析器。
    ///
    /// 参数 `path` 对应 Java `path`，可表示文件路径或自定义解析器名称。
    /// 对应 Java: `FlowParserFactory#createJsonELParser`。
    fn create_json_el_parser(&self, path: &str) -> LFResult<Box<dyn FlowParser>>;

    /// 创建 XML EL 解析器。
    ///
    /// 对应 Java: `FlowParserFactory#createXmlELParser`。
    fn create_xml_el_parser(&self, path: &str) -> LFResult<Box<dyn FlowParser>>;

    /// 创建 YML EL 解析器。
    ///
    /// 对应 Java: `FlowParserFactory#createYmlELParser`。
    fn create_yml_el_parser(&self, path: &str) -> LFResult<Box<dyn FlowParser>>;
}
