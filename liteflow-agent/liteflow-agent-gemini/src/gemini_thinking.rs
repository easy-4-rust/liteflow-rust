/// Gemini 平台 thinking 子构建器。
///
/// Gemini 2.5 使用 `thinking_level`，旧接口使用 token 数形式的 `thinking_budget`。
///
/// 对应 Java: `com.yomahub.liteflow.agent.gemini.GeminiThinking`。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GeminiThinking {
    level: Option<String>,
    budget: Option<u32>,
}

impl GeminiThinking {
    /// 设置 thinking 等级，例如 `low`、`medium`、`high`。
    ///
    /// 对应 Java: `GeminiThinking#level`。
    pub fn level(&mut self, level: impl Into<String>) -> &mut Self {
        self.level = Some(level.into());
        self
    }

    /// 设置 thinking token 预算。
    ///
    /// 对应 Java: `GeminiThinking#budget`。
    pub fn budget(&mut self, tokens: u32) -> &mut Self {
        self.budget = Some(tokens);
        self
    }

    /// 返回 thinking 等级。对应 Java: `GeminiThinking#getLevel`。
    #[must_use]
    pub fn get_level(&self) -> Option<&str> {
        self.level.as_deref()
    }

    /// 返回 thinking token 预算。对应 Java: `GeminiThinking#getBudget`。
    #[must_use]
    pub fn get_budget(&self) -> Option<u32> {
        self.budget
    }
}
