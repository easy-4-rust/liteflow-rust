//! Etcd XML EL 解析器。

use liteflow_core::rule_plugin::RuleSourceWatcher;

use super::exception::EtcdException;
use super::util::EtcdParserHelper;
use super::vo::EtcdParserVO;

/// 仅支持 EL 形式 XML 的 Etcd 解析器。
///
/// 对应 Java: `com.yomahub.liteflow.parser.etcd.EtcdXmlELParser`。
#[derive(Debug, Clone)]
pub struct EtcdXmlELParser {
    config: EtcdParserVO,
    helper: EtcdParserHelper,
}

impl EtcdXmlELParser {
    /// 校验扩展配置并创建解析器。
    ///
    /// 对应 Java `EtcdXmlELParser#EtcdXmlELParser`。
    pub fn new(config: EtcdParserVO) -> Result<Self, EtcdException> {
        config.validate()?;
        let helper = EtcdParserHelper::new(config.clone())?;
        Ok(Self { config, helper })
    }

    /// 聚合 Chain/Script 前缀树为 XML。
    ///
    /// 对应 Java `EtcdXmlELParser#parseCustom`。
    pub async fn parse_custom(&self) -> Result<String, EtcdException> {
        self.helper.get_content().await
    }

    /// 安装 Etcd Watch。参数 `watcher` 负责变更后的统一重载与删除对账。
    pub async fn listen(&self, watcher: RuleSourceWatcher) -> Result<(), EtcdException> {
        self.helper.listen(watcher).await
    }

    /// 返回解析器配置。
    #[must_use]
    pub fn config(&self) -> &EtcdParserVO {
        &self.config
    }

    /// 返回 Etcd 辅助对象。
    #[must_use]
    pub fn helper(&self) -> &EtcdParserHelper {
        &self.helper
    }
}
