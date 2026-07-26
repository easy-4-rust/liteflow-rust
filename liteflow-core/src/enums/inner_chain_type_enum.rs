//! 对应 com.yomahub.liteflow.enums.InnerChainTypeEnum：
//! 匿名/内部链路的执行环境标记（execute2RespWithEL 等场景）。

/// 内部链路类型枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum InnerChainTypeEnum {
    /// 不是隐式 chain
    None,
    /// 在串行环境中执行
    InSync,
    /// 在并行环境中执行
    InAsync,
}
