//! 对应 Java: com.yomahub.liteflow.spring.DeclBeanDefinition

use std::sync::atomic::{AtomicBool, Ordering};

use liteflow_core::LFResult;
use liteflow_core::core::proxy::{DeclWarpBean, LiteFlowProxyUtil};
use liteflow_core::spi::DeclComponentParserHolder;

use crate::{LiteflowComponentRegistration, VernalAware};

/// Vernal 声明式组件定义拆分器。
///
/// Java 版本在 BeanDefinition 注册阶段扫描 `@LiteflowMethod` 并把一个原始
/// BeanDefinition 拆成多个以节点 ID 命名的 `DeclWarpBean`。Rust 不做运行期
/// classpath 反射：`liteflow-derive` 已在编译期生成 `DeclWarpBean`，本对象保留
/// 后半段真实语义，调用容器声明解析 SPI、逐个注册拆分结果，并允许同名定义以后
/// 注册者覆盖前者。
///
/// 对应 Java: `com.yomahub.liteflow.spring.DeclBeanDefinition`。
#[derive(Debug, Default)]
pub struct VernalDeclBeanDefinition {
    bean_factory_post_processed: AtomicBool,
}

impl VernalDeclBeanDefinition {
    /// 创建 Vernal 声明式组件定义拆分器。
    ///
    /// # 返回
    /// 无状态、可在线程间共享的定义后处理器。对应 Java:
    /// `DeclBeanDefinition#DeclBeanDefinition`。
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// 解析并注册全部声明式组件定义。
    ///
    /// # 参数
    /// - `registrations`：模块构建期收集的原始组件定义；
    /// - `context_aware`：接收按节点 ID 命名的 `DeclWarpBean` 的 Vernal 容器适配器。
    ///
    /// # 返回
    /// 普通定义原样保留；声明式定义按解析器返回结果展开为零到多个真实注册动作。
    /// 同名 Bean 使用后注册覆盖语义。对应 Java:
    /// `DeclBeanDefinition#postProcessBeanDefinitionRegistry`。
    pub fn post_process_bean_definition_registry(
        &self,
        registrations: &[LiteflowComponentRegistration],
        context_aware: &VernalAware,
    ) -> LFResult<Vec<LiteflowComponentRegistration>> {
        let parser = DeclComponentParserHolder::load_decl_component_parser();
        let mut processed_registrations = Vec::new();

        for registration in registrations {
            let Some(decl_warp_bean) = registration.decl_warp_bean() else {
                processed_registrations.push(registration.clone());
                continue;
            };
            if !Self::has_liteflow_method_annotation(&decl_warp_bean) {
                continue;
            }

            // Java 允许一个源类按 @LiteflowMethod 的 nodeId 分裂成多个包装定义；
            // Rust 解析 SPI 同样返回列表，因此不能假定一个输入只产生一个节点。
            for parsed_decl_warp_bean in parser.parse_decl_bean(decl_warp_bean)? {
                self.register_new_bean_definition(context_aware, &parsed_decl_warp_bean);
                processed_registrations.push(LiteflowComponentRegistration::parsed_declarative(
                    parsed_decl_warp_bean,
                ));
            }
        }

        Ok(processed_registrations)
    }

    /// 执行 BeanFactory 后处理阶段。
    ///
    /// Vernal 组件定义在模块配置阶段已具备完整类型信息，因此该阶段与 Java
    /// 不再修改容器，只发布该生命周期阶段已经到达的只读诊断状态。
    ///
    /// 对应 Java: `DeclBeanDefinition#postProcessBeanFactory`。
    pub fn post_process_bean_factory(&self) {
        self.bean_factory_post_processed
            .store(true, Ordering::Release);
    }

    /// 返回 BeanFactory 后处理阶段是否已经到达。
    ///
    /// # 返回
    /// `post_process_bean_factory` 调用后返回 `true`。这是 Rust 容器启动诊断，
    /// 不向 Java 的空后处理阶段附加 Bean 修改行为。
    #[must_use]
    pub fn is_bean_factory_post_processed(&self) -> bool {
        self.bean_factory_post_processed.load(Ordering::Acquire)
    }

    fn has_liteflow_method_annotation(decl_warp_bean: &DeclWarpBean) -> bool {
        LiteFlowProxyUtil::is_declare_cmp(decl_warp_bean)
    }

    fn register_new_bean_definition(
        &self,
        context_aware: &VernalAware,
        decl_warp_bean: &DeclWarpBean,
    ) {
        // DefaultListableBeanFactory 的 allowBeanDefinitionOverriding(true) 映射为
        // VernalAware 命名表的 insert 覆盖，包装对象仍持有原始业务 Arc。
        context_aware.register_decl_wrap_bean(decl_warp_bean.node_id(), decl_warp_bean.clone());
    }
}
