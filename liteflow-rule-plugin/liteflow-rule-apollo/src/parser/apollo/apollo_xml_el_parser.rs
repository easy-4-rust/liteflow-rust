//! Apollo XML EL 解析器。

use super::exception::ApolloException;
use super::util::ApolloParseHelper;
use super::vo::ApolloParserConfigVO;

/// 校验 Apollo 扩展配置并委托辅助对象生成 LiteFlow XML。
///
/// 对应 Java: `com.yomahub.liteflow.parser.apollo.ApolloXmlELParser`。
#[derive(Debug, Clone)]
pub struct ApolloXmlELParser {
    config: ApolloParserConfigVO,
    helper: ApolloParseHelper,
}

impl ApolloXmlELParser {
    /// 使用扩展配置和 Apollo Config Service 连接参数创建解析器。
    ///
    /// `chain_namespace` 为空时与 Java 构造函数一样返回配置异常。
    /// 对应 Java: `ApolloXmlELParser#ApolloXmlELParser`。
    pub fn new(
        config: ApolloParserConfigVO,
        config_service_url: impl Into<String>,
        app_id: impl Into<String>,
        cluster: impl Into<String>,
    ) -> Result<Self, ApolloException> {
        config.validate()?;
        let helper = ApolloParseHelper::new(config.clone(), config_service_url, app_id, cluster)?;
        Ok(Self { config, helper })
    }

    /// 设置 Config Service 请求中的客户端 IP。
    #[must_use]
    pub fn with_ip(mut self, ip: impl Into<String>) -> Self {
        self.helper = self.helper.with_ip(ip);
        self
    }

    /// 聚合 Apollo 中全部 Chain 与可选 Script 配置。
    ///
    /// 返回值为 Java `ClassXmlFlowELParser` 消费的 XML 文本。
    /// 对应 Java: `ApolloXmlELParser#parseCustom`。
    pub fn parse_custom(&self) -> Result<String, ApolloException> {
        self.helper.get_content()
    }

    /// 返回解析器扩展配置。
    #[must_use]
    pub fn config(&self) -> &ApolloParserConfigVO {
        &self.config
    }

    /// 返回 Config Service 根地址。
    #[must_use]
    pub fn config_service_url(&self) -> &str {
        self.helper.config_service_url()
    }

    /// 返回 Apollo 应用 id。
    #[must_use]
    pub fn app_id(&self) -> &str {
        self.helper.app_id()
    }

    /// 返回 Apollo 集群名称。
    #[must_use]
    pub fn cluster(&self) -> &str {
        self.helper.cluster()
    }

    /// 返回 Config Service 客户端 IP。
    #[must_use]
    pub fn ip(&self) -> &str {
        self.helper.ip()
    }

    /// 返回 Apollo 解析辅助对象。
    #[must_use]
    pub fn helper(&self) -> &ApolloParseHelper {
        &self.helper
    }
}
