/// Anthropic 平台 thinking 子构建器，沿用原生 budget/enabled 术语。
///
/// 对应 Java: `com.yomahub.liteflow.agent.anthropic.AnthropicThinking`。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AnthropicThinking {
    budget: Option<u32>,
    enabled: Option<bool>,
}

impl AnthropicThinking {
    /// 设置 thinking token 预算。
    ///
    /// 对应 Java: `AnthropicThinking#budget`。
    pub fn budget(&mut self, tokens: u32) -> &mut Self {
        self.budget = Some(tokens);
        self
    }

    /// 设置 thinking 是否启用。
    ///
    /// 对应 Java: `AnthropicThinking#enabled`。
    pub fn enabled(&mut self, value: bool) -> &mut Self {
        self.enabled = Some(value);
        self
    }

    /// 返回 thinking token 预算。对应 Java: `AnthropicThinking#getBudget`。
    #[must_use]
    pub fn get_budget(&self) -> Option<u32> {
        self.budget
    }

    /// 返回 thinking 启用状态。对应 Java: `AnthropicThinking#getEnabled`。
    #[must_use]
    pub fn get_enabled(&self) -> Option<bool> {
        self.enabled
    }
}
