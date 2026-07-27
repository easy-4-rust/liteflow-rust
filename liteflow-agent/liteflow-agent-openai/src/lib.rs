//! LiteFlow Agent OpenAI 兼容模型适配。

pub mod model;

pub use model::{
    DeepSeek, Glm, Kimi, Minimax, OpenAi, OpenAiAgentModelConfig, OpenAiCompatible,
    OpenAiCompatiblePresets, OpenAiCompatibleSpec, OpenAiModelFactory, OpenAiSpec,
};
