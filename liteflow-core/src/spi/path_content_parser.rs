//! 对应 Java 类：com.yomahub.liteflow.spi.PathContentParser
//!
//! 路径内容解析 SPI 接口。
//! Java 支持 classpath 路径与 file 绝对路径（spring 环境支持
//! PathMatchingResourcePatternResolver 规则）。Rust 无运行期 classpath：
//! `classpath:` 语义对应 include_str!/构建期嵌入，运行期不支持
//! （见 local::LocalPathContentParser 的注释说明）。

use crate::exception::LFResult;

use super::spi_priority::SpiPriority;

/// 对应 PathContentParser
pub trait PathContentParser: SpiPriority + Send + Sync {
    /// 对应 parseContent(List<String> pathList)：解析路径下的文件内容
    fn parse_content(&self, path_list: &[String]) -> LFResult<Vec<String>>;

    /// 对应 getFileAbsolutePath(List<String> pathList)：获取文件的绝对路径
    fn get_file_absolute_path(&self, path_list: &[String]) -> LFResult<Vec<String>>;
}
