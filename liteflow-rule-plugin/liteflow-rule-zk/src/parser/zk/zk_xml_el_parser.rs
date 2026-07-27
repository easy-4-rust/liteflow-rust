//! ZooKeeper XML EL 解析器。

use liteflow_core::rule_plugin::RuleSourceWatcher;

use super::exception::ZkException;
use super::util::ZkParserHelper;
use super::vo::ZkParserVO;

/// 仅支持 EL 形式 XML 的 ZooKeeper 解析器。
///
/// 对应 Java: `com.yomahub.liteflow.parser.zk.ZkXmlELParser`。
#[derive(Clone)]
pub struct ZkXmlELParser {
    config: ZkParserVO,
    helper: ZkParserHelper,
}

impl ZkXmlELParser {
    /// 校验扩展配置并创建 ZooKeeper 解析器。
    ///
    /// 对应 Java `ZkXmlELParser#ZkXmlELParser`。
    pub fn new(config: ZkParserVO) -> Result<Self, ZkException> {
        config.validate()?;
        let helper = ZkParserHelper::new(config.clone())?;
        Ok(Self { config, helper })
    }

    /// 聚合 Chain/Script 子节点为 XML。
    ///
    /// 对应 Java `ZkXmlELParser#parseCustom`。
    pub fn parse_custom(&self) -> Result<String, ZkException> {
        self.helper.get_content()
    }

    /// 安装 ZooKeeper 原生持久递归 Watch。
    pub fn listen(&self, watcher: RuleSourceWatcher) -> Result<(), ZkException> {
        self.helper.listen_zk_node(watcher)
    }

    /// 返回解析器配置。
    #[must_use]
    pub fn config(&self) -> &ZkParserVO {
        &self.config
    }

    /// 返回 ZooKeeper 辅助对象。
    #[must_use]
    pub fn helper(&self) -> &ZkParserHelper {
        &self.helper
    }
}
