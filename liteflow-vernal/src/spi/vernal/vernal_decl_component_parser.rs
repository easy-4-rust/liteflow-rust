//! 对应 Java 类：com.yomahub.liteflow.spi.spring.SpringDeclComponentParser

use liteflow_core::core::proxy::DeclWarpBean;
use liteflow_core::exception::{CmpDefinitionException, LFResult};
use liteflow_core::spi::{DeclComponentParser, SpiPriority};

/// Vernal 环境中的声明式组件解析器。
///
/// Java 实现扫描 `Class#getMethods` 并读取 `@LiteflowMethod`、
/// `@LiteflowRetry`、`@LiteflowFact` 等注解；Rust 不在运行期反射，以上信息由
/// `liteflow-derive` 在编译期写入 `DeclWarpBean`。本解析器负责 Spring 实现后半段
/// 的等价职责：过滤空节点 ID、确认存在主处理方法、以主方法确定节点类型，并
/// 保留同一个 Vernal 单例业务对象。
///
/// 对应 Java: `com.yomahub.liteflow.spi.spring.SpringDeclComponentParser`。
#[derive(Debug, Default)]
pub struct VernalDeclComponentParser;

impl VernalDeclComponentParser {
    /// 创建 Vernal 声明式组件解析器。
    ///
    /// # 返回
    /// 无状态、可在线程间共享的解析器。对应 Java:
    /// `SpringDeclComponentParser#SpringDeclComponentParser`。
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// 解析编译期生成的声明式组件元数据。
    ///
    /// # 参数
    /// - `decl_warp_bean`：包含节点、原始单例对象和全部声明式方法的包装对象。
    ///
    /// # 返回
    /// 节点 ID 非空且含主处理方法时返回单元素列表；空 ID 与 Java `filter`
    /// 行为一致返回空列表；缺少主处理方法时返回 `CmpDefinitionException` 对应
    /// 错误。对应 Java: `SpringDeclComponentParser#parseDeclBean(Class)`。
    pub fn parse_decl_bean(&self, mut decl_warp_bean: DeclWarpBean) -> LFResult<Vec<DeclWarpBean>> {
        if decl_warp_bean.node_id().trim().is_empty() {
            return Ok(Vec::new());
        }

        // Java 从同一 nodeId 分组中寻找 isMainMethod；Rust 包装对象已经在编译期
        // 完成分组，因此只需在真实方法元数据中完成相同校验。
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

        // Spring 版本以主方法所在 DeclInfo 的 nodeType 写回 DeclWarpBean；
        // Rust 同样让主方法成为节点类型的权威来源，后续代理仍会校验所有方法一致。
        decl_warp_bean.set_node_type(process_method.node_type());
        Ok(vec![decl_warp_bean])
    }

    /// 使用显式节点 ID 与名称解析声明式组件。
    ///
    /// # 参数
    /// - `decl_warp_bean`：编译期生成的声明式组件包装对象；
    /// - `node_id`：调用方覆盖的节点 ID；
    /// - `node_name`：调用方覆盖的节点显示名。
    ///
    /// # 返回
    /// 身份覆盖后执行与普通入口相同的校验。对应 Java:
    /// `SpringDeclComponentParser#parseDeclBean(Class,String,String)`。
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

    /// 返回 Vernal 容器实现的 SPI 优先级。
    ///
    /// # 返回
    /// 固定返回 `1`，优先于本地实现。对应 Java:
    /// `SpringDeclComponentParser#priority`。
    #[must_use]
    pub fn priority(&self) -> i32 {
        1
    }
}

impl DeclComponentParser for VernalDeclComponentParser {
    fn parse_decl_bean(&self, decl_warp_bean: DeclWarpBean) -> LFResult<Vec<DeclWarpBean>> {
        VernalDeclComponentParser::parse_decl_bean(self, decl_warp_bean)
    }

    fn parse_decl_bean_with_identity(
        &self,
        decl_warp_bean: DeclWarpBean,
        node_id: &str,
        node_name: &str,
    ) -> LFResult<Vec<DeclWarpBean>> {
        VernalDeclComponentParser::parse_decl_bean_with_identity(
            self,
            decl_warp_bean,
            node_id,
            node_name,
        )
    }
}

impl SpiPriority for VernalDeclComponentParser {
    fn priority(&self) -> i32 {
        VernalDeclComponentParser::priority(self)
    }
}
