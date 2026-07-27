//! SQL Parser 类名 SPI。

/// 返回 SQL XML EL 解析器的稳定 Rust 类型路径。
///
/// Rust 编译期直接引用类型，不需要 Java ServiceLoader 的反射类名；该对象保留
/// ParserClassNameSpi 的发现契约。对应 Java:
/// `com.yomahub.liteflow.parser.spi.sql.SQLParserClassNameSpi`。
#[derive(Debug, Clone, Copy, Default)]
pub struct SQLParserClassNameSpi;

impl SQLParserClassNameSpi {
    /// 返回 SQL 解析器类型路径。对应 Java `getSpiClassName()`。
    #[must_use]
    pub const fn get_spi_class_name(&self) -> &'static str {
        "liteflow_rule_sql::SQLXmlELParser"
    }
}
