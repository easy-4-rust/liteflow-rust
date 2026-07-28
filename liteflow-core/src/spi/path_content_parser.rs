//! 对应 Java 类：com.yomahub.liteflow.spi.PathContentParser
//!
//! 路径内容解析 SPI 接口。
//! Java 支持 classpath 路径与 file 绝对路径（spring 环境支持
//! PathMatchingResourcePatternResolver 规则）。Rust 本地实现以 `CLASSPATH`
//! 环境变量、应用工作目录、Cargo 资源约定目录和可执行文件相邻资源目录组成
//! 运行时资源根，实现 Java ClassLoader 等价的 `classpath:` 查找。

use crate::exception::LFResult;

use super::spi_priority::SpiPriority;

/// 规则路径内容解析 SPI。
///
/// 对应 Java: `com.yomahub.liteflow.spi.PathContentParser`。
pub trait PathContentParser: SpiPriority + Send + Sync {
    /// 解析路径下的文件内容。
    ///
    /// 参数 `path_list` 支持 classpath 资源和 file 绝对路径；返回所有非空资源
    /// 内容。对应 Java: `PathContentParser#parseContent(List<String>)`。
    fn parse_content(&self, path_list: &[String]) -> LFResult<Vec<String>>;

    /// 获取路径对应文件的绝对路径。
    ///
    /// 参数 `path_list` 与 `parse_content` 语义一致；返回当前可解析资源的绝对
    /// 文件路径。对应 Java:
    /// `PathContentParser#getFileAbsolutePath(List<String>)`。
    fn get_file_absolute_path(&self, path_list: &[String]) -> LFResult<Vec<String>>;
}
