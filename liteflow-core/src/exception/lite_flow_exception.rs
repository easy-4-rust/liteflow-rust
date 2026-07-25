//! 对应 Java 类：com.yomahub.liteflow.exception.LiteFlowException

/// LiteFlow 异常基础 trait。
///
/// 所有 LiteFlow 异常类型都应实现此 trait。
pub trait LiteFlowException: std::error::Error {
    /// 获取异常消息。
    fn message(&self) -> &str;

    /// 获取异常类型名称。
    fn exception_type(&self) -> &'static str {
        std::any::type_name::<Self>()
    }
}

/// LiteFlow 错误枚举，包含所有可能的错误类型。
#[derive(Debug, Clone)]
pub enum LiteFlowError {
    /// 通用错误
    General(String),
    /// 解析错误
    Parse(String),
    /// 执行错误
    Execute(String),
    /// 配置错误
    Config(String),
    /// 未找到错误
    NotFound(String),
    /// 超时错误
    Timeout(String),
}

impl std::fmt::Display for LiteFlowError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LiteFlowError::General(msg) => write!(f, "General error: {}", msg),
            LiteFlowError::Parse(msg) => write!(f, "Parse error: {}", msg),
            LiteFlowError::Execute(msg) => write!(f, "Execute error: {}", msg),
            LiteFlowError::Config(msg) => write!(f, "Config error: {}", msg),
            LiteFlowError::NotFound(msg) => write!(f, "Not found: {}", msg),
            LiteFlowError::Timeout(msg) => write!(f, "Timeout: {}", msg),
        }
    }
}

impl std::error::Error for LiteFlowError {}

/// LiteFlow Result 类型别名。
pub type LiteFlowResult<T> = Result<T, LiteFlowError>;

/// 从字符串创建通用错误。
pub fn general_error(msg: impl Into<String>) -> LiteFlowError {
    LiteFlowError::General(msg.into())
}

/// 从字符串创建解析错误。
pub fn parse_error(msg: impl Into<String>) -> LiteFlowError {
    LiteFlowError::Parse(msg.into())
}

/// 从字符串创建执行错误。
pub fn execute_error(msg: impl Into<String>) -> LiteFlowError {
    LiteFlowError::Execute(msg.into())
}

/// 从字符串创建配置错误。
pub fn config_error(msg: impl Into<String>) -> LiteFlowError {
    LiteFlowError::Config(msg.into())
}

/// 从字符串创建未找到错误。
pub fn not_found_error(msg: impl Into<String>) -> LiteFlowError {
    LiteFlowError::NotFound(msg.into())
}

/// 从字符串创建超时错误。
pub fn timeout_error(msg: impl Into<String>) -> LiteFlowError {
    LiteFlowError::Timeout(msg.into())
}
