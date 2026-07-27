//! LiteFlow Agent AWS Bedrock（SigV4 签名）模型适配（衍生自 ZeroClaw，Apache-2.0）。

pub mod model;
pub mod provider;

pub use model::BedrockAgentModelConfig;
pub use provider::{AwsCredentials, BedrockProvider};
