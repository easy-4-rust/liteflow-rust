use std::sync::Arc;

use crate::spi::{SpiFactory, path_content_parser::PathContentParser};

/// 路径内容解析器持有者
///
/// 用于获取路径内容解析器实例
pub struct PathContentParserHolder {
    /// 路径内容解析器实例
    path_content_parser: Arc<dyn PathContentParser>,
}

impl PathContentParserHolder {
    /// 创建路径内容解析器持有者
    ///
    /// 通过 SPI 工厂获取路径内容解析器实例
    pub fn new() -> Self {
        let path_content_parser = SpiFactory::path_content_parser();
        Self {
            path_content_parser,
        }
    }

    /// 获取路径内容解析器实例
    ///
    /// 返回路径内容解析器实例
    pub fn path_content_parser(&self) -> Arc<dyn PathContentParser> {
        self.path_content_parser.clone()
    }
}
