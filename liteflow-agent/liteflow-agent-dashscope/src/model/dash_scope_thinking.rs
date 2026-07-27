/// DashScope（通义千问）thinking 子构建器。
///
/// 对应 Java: `com.yomahub.liteflow.agent.dashscope.DashScopeThinking`。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DashScopeThinking {
    budget: Option<u32>,
}

impl DashScopeThinking {
    /// 设置 thinking token 预算。
    ///
    /// 对应 Java: `DashScopeThinking#budget`。
    pub fn budget(&mut self, tokens: u32) -> &mut Self {
        self.budget = Some(tokens);
        self
    }

    /// 返回 thinking token 预算。对应 Java: `DashScopeThinking#getBudget`。
    #[must_use]
    pub fn get_budget(&self) -> Option<u32> {
        self.budget
    }
}
