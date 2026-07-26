use std::fmt;

/// 用于 EL 构建器中设置 `retry` 关键字的值对象。
///
/// 对应 Java: `com.yomahub.liteflow.builder.el.vo.RetryELVo`。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetryELVo {
    count: u32,
    exceptions: Vec<String>,
}

impl RetryELVo {
    /// 创建仅包含重试次数的值对象。对应 Java: `RetryELVo#RetryELVo(int)`。
    ///
    /// # 参数
    /// - `count`: 失败后的最大重试次数。
    ///
    /// # 返回
    /// 新的重试值对象。
    pub fn new(count: u32) -> Self {
        Self {
            count,
            exceptions: Vec::new(),
        }
    }

    /// 创建包含重试次数和异常类型名的值对象。
    /// 对应 Java: `RetryELVo#RetryELVo(int, String...)`。
    ///
    /// # 参数
    /// - `count`: 失败后的最大重试次数。
    /// - `exceptions`: 允许触发重试的异常类型名。
    ///
    /// # 返回
    /// 新的重试值对象。
    pub fn with_exceptions<I, S>(count: u32, exceptions: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            count,
            exceptions: exceptions.into_iter().map(Into::into).collect(),
        }
    }

    /// 返回最大重试次数。
    pub fn count(&self) -> u32 {
        self.count
    }

    /// 返回异常类型名列表。
    pub fn exceptions(&self) -> &[String] {
        &self.exceptions
    }

    pub(crate) fn runtime_text(&self) -> String {
        self.count.to_string()
    }
}

impl fmt::Display for RetryELVo {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.count)?;
        for exception in &self.exceptions {
            write!(
                formatter,
                ",\"{}\"",
                super::super::el_wrapper::escape_el_string(exception)
            )?;
        }
        Ok(())
    }
}
