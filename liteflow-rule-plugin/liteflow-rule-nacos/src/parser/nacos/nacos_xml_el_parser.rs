//! Nacos XML EL 解析器。

use liteflow_core::rule_plugin::RuleSourceWatcher;

use super::exception::NacosException;
use super::util::NacosParseHelper;
use super::vo::NacosParserVO;

/// 仅支持 EL 形式 XML 的 Nacos 解析器。
///
/// 对应 Java: `com.yomahub.liteflow.parser.nacos.NacosXmlELParser`。
#[derive(Debug, Clone)]
pub struct NacosXmlELParser {
    config: NacosParserVO,
    helper: NacosParseHelper,
}

impl NacosXmlELParser {
    /// 校验 Nacos 扩展配置并创建解析器。
    ///
    /// 对应 Java `NacosXmlELParser#NacosXmlELParser`。
    pub fn new(config: NacosParserVO) -> Result<Self, NacosException> {
        config.validate()?;
        let helper = NacosParseHelper::new(config.clone())?;
        Ok(Self { config, helper })
    }

    /// 读取并校验 XML EL 规则内容。
    ///
    /// 对应 Java `NacosXmlELParser#parseCustom`。
    pub async fn parse_custom(&self) -> Result<String, NacosException> {
        let content = self.helper.get_content().await?;
        self.helper.check_content(&content)?;
        Ok(content)
    }

    /// 安装 Nacos 原生配置变化监听。
    ///
    /// 对应 Java `NacosXmlELParser#parseCustom` 中的监听注册。
    pub async fn listen(&self, watcher: RuleSourceWatcher) -> Result<(), NacosException> {
        self.helper.listener(watcher).await
    }

    /// 返回解析器配置。
    #[must_use]
    pub fn config(&self) -> &NacosParserVO {
        &self.config
    }

    /// 返回 Nacos 解析辅助对象。
    #[must_use]
    pub fn helper(&self) -> &NacosParseHelper {
        &self.helper
    }
}
