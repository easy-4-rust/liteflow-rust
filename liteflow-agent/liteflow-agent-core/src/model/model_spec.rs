use agentscope_core::model::GenerateOptions;
use serde::{Deserialize, Serialize};

/// Provider-neutral 模型共性参数描述符。
///
/// Java 通过抽象基类让各 Provider Spec 继承这些字段；Rust 由各具体 Provider Spec
/// 组合本对象，并在自身的 `resolve` 中完成凭证读取和 AgentScope 模型构建。
///
/// 对应 Java: `com.yomahub.liteflow.agent.model.ModelSpec`。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ModelSpec {
    temperature: Option<f64>,
    top_p: Option<f64>,
    top_k: Option<u32>,
    max_tokens: Option<u32>,
    seed: Option<i64>,
    stream: Option<bool>,
    cache_control: Option<bool>,
}

impl ModelSpec {
    /// 创建所有共性参数均未指定的模型描述符。
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置采样温度并返回更新后的描述符。
    ///
    /// 对应 Java: `ModelSpec#temperature`。
    #[must_use]
    pub fn temperature(mut self, value: f64) -> Self {
        self.temperature = Some(value);
        self
    }

    /// 设置核采样概率并返回更新后的描述符。
    ///
    /// 对应 Java: `ModelSpec#topP`。
    #[must_use]
    pub fn top_p(mut self, value: f64) -> Self {
        self.top_p = Some(value);
        self
    }

    /// 设置 Top-K 采样参数并返回更新后的描述符。
    ///
    /// 对应 Java: `ModelSpec#topK`。
    #[must_use]
    pub fn top_k(mut self, value: u32) -> Self {
        self.top_k = Some(value);
        self
    }

    /// 设置最大输出 token 数并返回更新后的描述符。
    ///
    /// 对应 Java: `ModelSpec#maxTokens`。
    #[must_use]
    pub fn max_tokens(mut self, value: u32) -> Self {
        self.max_tokens = Some(value);
        self
    }

    /// 设置随机种子并返回更新后的描述符。
    ///
    /// 对应 Java: `ModelSpec#seed`。
    #[must_use]
    pub fn seed(mut self, value: i64) -> Self {
        self.seed = Some(value);
        self
    }

    /// 设置是否请求流式响应并返回更新后的描述符。
    ///
    /// 该字段由具体 Provider 的模型 Builder 消费，不重复放入共性
    /// `GenerateOptions`。
    ///
    /// 对应 Java: `ModelSpec#stream`。
    #[must_use]
    pub fn stream(mut self, value: bool) -> Self {
        self.stream = Some(value);
        self
    }

    /// 设置是否启用提示缓存控制并返回更新后的描述符。
    ///
    /// 对应 Java: `ModelSpec#cacheControl`。
    #[must_use]
    pub fn cache_control(mut self, value: bool) -> Self {
        self.cache_control = Some(value);
        self
    }

    /// 返回采样温度。对应 Java: `ModelSpec#getTemperature`。
    #[must_use]
    pub fn get_temperature(&self) -> Option<f64> {
        self.temperature
    }

    /// 返回核采样概率。对应 Java: `ModelSpec#getTopP`。
    #[must_use]
    pub fn get_top_p(&self) -> Option<f64> {
        self.top_p
    }

    /// 返回 Top-K。对应 Java: `ModelSpec#getTopK`。
    #[must_use]
    pub fn get_top_k(&self) -> Option<u32> {
        self.top_k
    }

    /// 返回最大输出 token 数。对应 Java: `ModelSpec#getMaxTokens`。
    #[must_use]
    pub fn get_max_tokens(&self) -> Option<u32> {
        self.max_tokens
    }

    /// 返回随机种子。对应 Java: `ModelSpec#getSeed`。
    #[must_use]
    pub fn get_seed(&self) -> Option<i64> {
        self.seed
    }

    /// 返回可选流式开关。对应 Java: `ModelSpec#getStream`。
    #[must_use]
    pub fn get_stream(&self) -> Option<bool> {
        self.stream
    }

    /// 返回可选缓存控制开关。对应 Java: `ModelSpec#getCacheControl`。
    #[must_use]
    pub fn get_cache_control(&self) -> Option<bool> {
        self.cache_control
    }

    /// 把已设置的共性生成参数转换为 AgentScope `GenerateOptions`。
    ///
    /// `stream` 是模型连接/传输行为，由具体 Provider Builder 单独读取；其余参数只有
    /// 至少一项被设置时才返回 `Some`，对齐 Java Provider Spec 的空值判断。
    #[must_use]
    pub fn generate_options(&self) -> Option<GenerateOptions> {
        if self.temperature.is_none()
            && self.top_p.is_none()
            && self.top_k.is_none()
            && self.max_tokens.is_none()
            && self.seed.is_none()
            && self.cache_control.is_none()
        {
            return None;
        }

        Some(GenerateOptions {
            temperature: self.temperature,
            top_p: self.top_p,
            top_k: self.top_k,
            max_tokens: self.max_tokens,
            seed: self.seed,
            cache_control: self.cache_control,
            ..GenerateOptions::default()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::ModelSpec;

    #[test]
    fn common_parameters_map_to_generate_options_and_stream_stays_transport_level() {
        let empty = ModelSpec::new().stream(true);
        assert!(empty.generate_options().is_none());
        assert_eq!(empty.get_stream(), Some(true));

        let spec = ModelSpec::new()
            .temperature(0.2)
            .top_p(0.8)
            .top_k(40)
            .max_tokens(512)
            .seed(7)
            .cache_control(true)
            .stream(false);
        let options = spec
            .generate_options()
            .expect("任一共性生成参数存在时应构造 GenerateOptions");
        assert_eq!(options.temperature, Some(0.2));
        assert_eq!(options.top_p, Some(0.8));
        assert_eq!(options.top_k, Some(40));
        assert_eq!(options.max_tokens, Some(512));
        assert_eq!(options.seed, Some(7));
        assert_eq!(options.cache_control, Some(true));
        assert_eq!(spec.get_stream(), Some(false));
    }
}
