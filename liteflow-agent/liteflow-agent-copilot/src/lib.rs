//! LiteFlow Agent GitHub Copilot（OAuth 设备流）模型适配（衍生自 ZeroClaw，Apache-2.0）。

pub mod model;
pub mod provider;

pub use model::CopilotAgentModelConfig;
pub use provider::CopilotProvider;
