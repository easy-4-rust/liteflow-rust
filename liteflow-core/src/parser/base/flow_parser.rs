//! 规则解析器统一接口。

use crate::exception::LFResult;

/// 所有规则解析器必须实现的统一协议。
///
/// Rust 端额外返回本次成功装载的 chain id，便于调用方验证平滑加载结果；
/// Java 端对应方法返回 `void`，其余输入与异常语义保持一致。
///
/// 对应 Java: `com.yomahub.liteflow.parser.base.FlowParser`。
pub trait FlowParser: Send + Sync {
    /// 从路径列表读取并解析规则。
    ///
    /// 参数 `path_list` 对应 Java `pathList`；返回本次成功装载的 chain id。
    /// 对应 Java: `FlowParser#parseMain`。
    fn parse_main(&self, path_list: &[String]) -> LFResult<Vec<String>>;

    /// 解析已经读取到内存的规则文本列表。
    ///
    /// 参数 `content_list` 对应 Java `contentList`；返回本次成功装载的 chain id。
    /// 对应 Java: `FlowParser#parse`。
    fn parse(&self, content_list: &[String]) -> LFResult<Vec<String>>;
}
