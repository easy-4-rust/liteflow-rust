use liteflow_core::core::proxy::DeclWarpBean;
use liteflow_core::exception::{CmpDefinitionException, LFResult};
use liteflow_core::spi::{DeclComponentParser, SpiPriority};

/// Solon 环境的声明式组件解析器。
///
/// Java 运行期反射读取 `@LiteflowMethod`、`@LiteflowRetry`、`@LiteflowFact`、
/// `@LiteflowCmpDefine` 与 Solon `@Component`；Rust 由 `liteflow-derive` 在编译期
/// 写入 `DeclWarpBean`，本对象执行 Java 后半段的分组过滤、主方法校验、类型确定
/// 与显式身份覆盖。对应 Java:
/// `com.yomahub.liteflow.spi.solon.SolonDeclComponentParser`。
#[derive(Debug, Default)]
pub struct SolonDeclComponentParser;

impl SolonDeclComponentParser {
    /// 创建无状态的 Solon 声明式解析器。
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// 解析声明式组件包装对象。
    ///
    /// # 参数
    /// - `decl_warp_bean`：编译期生成的节点与方法元数据。
    ///
    /// # 返回
    /// 空节点 ID 返回空列表；存在主处理方法时返回校验后的单元素列表；缺少主
    /// 方法返回 `CmpDefinitionException`。对应 Java:
    /// `SolonDeclComponentParser#parseDeclBean(Class)`。
    pub fn parse_decl_bean(&self, mut decl_warp_bean: DeclWarpBean) -> LFResult<Vec<DeclWarpBean>> {
        if decl_warp_bean.node_id().trim().is_empty() {
            return Ok(Vec::new());
        }
        let process_method = decl_warp_bean
            .method_wrap_bean_list()
            .iter()
            .find(|method| method.liteflow_method().is_main_method())
            .cloned()
            .ok_or_else(|| {
                CmpDefinitionException::new(format!(
                    "Component [{}] does not define the process method",
                    decl_warp_bean.node_id()
                ))
            })?;
        // Java 以主方法所在 DeclInfo 的 nodeType 作为声明组件最终类型。
        decl_warp_bean.set_node_type(process_method.node_type());
        Ok(vec![decl_warp_bean])
    }

    /// 使用显式节点身份解析声明式组件。
    ///
    /// # 参数
    /// - `decl_warp_bean`：编译期元数据；
    /// - `node_id`：覆盖节点 ID；
    /// - `node_name`：覆盖节点显示名。
    ///
    /// # 返回
    /// 写入身份后执行同一主方法校验。对应 Java:
    /// `SolonDeclComponentParser#parseDeclBean(Class,String,String)`。
    pub fn parse_decl_bean_with_identity(
        &self,
        mut decl_warp_bean: DeclWarpBean,
        node_id: &str,
        node_name: &str,
    ) -> LFResult<Vec<DeclWarpBean>> {
        decl_warp_bean.set_node_id(node_id);
        decl_warp_bean.set_node_name(node_name);
        self.parse_decl_bean(decl_warp_bean)
    }

    /// 返回 Solon SPI 优先级。
    #[must_use]
    pub fn priority(&self) -> i32 {
        1
    }
}

impl DeclComponentParser for SolonDeclComponentParser {
    fn parse_decl_bean(&self, decl_warp_bean: DeclWarpBean) -> LFResult<Vec<DeclWarpBean>> {
        SolonDeclComponentParser::parse_decl_bean(self, decl_warp_bean)
    }

    fn parse_decl_bean_with_identity(
        &self,
        decl_warp_bean: DeclWarpBean,
        node_id: &str,
        node_name: &str,
    ) -> LFResult<Vec<DeclWarpBean>> {
        SolonDeclComponentParser::parse_decl_bean_with_identity(
            self,
            decl_warp_bean,
            node_id,
            node_name,
        )
    }
}

impl SpiPriority for SolonDeclComponentParser {
    fn priority(&self) -> i32 {
        SolonDeclComponentParser::priority(self)
    }
}
