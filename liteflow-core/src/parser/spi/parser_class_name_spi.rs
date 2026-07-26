//! 对应 Java: com.yomahub.liteflow.parser.spi.ParserClassNameSpi

/// 提供自定义规则解析器注册名称。
///
/// Java 返回类全名供反射加载；Rust 返回注册到 `ClassParserFactory` 的稳定名称。
pub trait ParserClassNameSpi: Send + Sync {
    /// 返回解析器 SPI 名称。对应 Java `getSpiClassName`。
    fn get_spi_class_name(&self) -> &str;
}
