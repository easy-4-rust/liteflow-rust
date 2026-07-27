//! ZooKeeper 解析器 SPI 名称提供器。

use liteflow_core::parser::ParserClassNameSpi;

/// 向解析器工厂暴露 ZooKeeper XML EL 解析器的稳定名称。
///
/// 对应 Java: `com.yomahub.liteflow.parser.spi.zk.ZkParserClassNameSpi`。
#[derive(Debug, Clone, Copy, Default)]
pub struct ZkParserClassNameSpi;

impl ParserClassNameSpi for ZkParserClassNameSpi {
    /// 返回 Java 对齐的解析器类全名。对应 Java `getSpiClassName`。
    fn get_spi_class_name(&self) -> &str {
        "com.yomahub.liteflow.parser.zk.ZkXmlELParser"
    }
}
