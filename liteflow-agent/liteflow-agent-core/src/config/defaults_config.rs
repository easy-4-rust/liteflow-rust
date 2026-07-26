use serde::{Deserialize, Serialize};

/// Agent 全局默认值配置，对应配置段 `liteflow.agent.defaults.*`。
///
/// 对应 Java: `com.yomahub.liteflow.property.agent.DefaultsConfig`。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct DefaultsConfig {
    /// ReAct 流程的默认最大 reason → act 迭代次数，用于避免模型无限循环。
    pub max_iterations: usize,
}

impl Default for DefaultsConfig {
    fn default() -> Self {
        Self { max_iterations: 50 }
    }
}

impl DefaultsConfig {
    /// 返回默认最大迭代次数。对应 Java: `DefaultsConfig#getMaxIterations`。
    #[must_use]
    pub fn max_iterations(&self) -> usize {
        self.max_iterations
    }

    /// 设置默认最大迭代次数。对应 Java: `DefaultsConfig#setMaxIterations`。
    pub fn set_max_iterations(&mut self, max_iterations: usize) {
        self.max_iterations = max_iterations;
    }
}
