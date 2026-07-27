//! Redis Parser 类名 SPI。

use liteflow_core::parser::spi::ParserClassNameSpi;

/// 向解析器工厂暴露 Redis XML EL Parser 的稳定注册名。
///
/// 对应 Java:
/// `com.yomahub.liteflow.parser.spi.redis.RedisParserClassNameSpi`。
pub struct RedisParserClassNameSpi;

impl ParserClassNameSpi for RedisParserClassNameSpi {
    /// 返回 Java 同名解析器的全限定名。
    ///
    /// 对应 Java `RedisParserClassNameSpi#getSpiClassName`。
    fn get_spi_class_name(&self) -> &str {
        "com.yomahub.liteflow.parser.redis.RedisXmlELParser"
    }
}

#[cfg(test)]
mod tests {
    use liteflow_core::parser::spi::ParserClassNameSpi;

    use super::RedisParserClassNameSpi;

    #[test]
    fn exposes_java_parser_class_name() {
        assert_eq!(
            RedisParserClassNameSpi.get_spi_class_name(),
            "com.yomahub.liteflow.parser.redis.RedisXmlELParser"
        );
    }
}
