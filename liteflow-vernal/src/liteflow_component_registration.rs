//! 对应 Java: `ComponentScanner` 对普通组件和声明式组件的注册结果。

use liteflow_core::{FlowBus, LFResult};

use crate::SharedRegistration;

/// 一个显式、可组合的 LiteFlow 组件注册动作。
///
/// Vernal 明确不做 classpath 扫描，因此 Rust 侧在应用模块构建期提交注册动作；
/// `liteflow-derive` 生成的组件注册方法可以直接包装到本对象中。
#[derive(Clone)]
pub struct LiteflowComponentRegistration {
    component_id: String,
    registration: SharedRegistration,
}

impl LiteflowComponentRegistration {
    /// 创建组件注册动作。
    pub fn new<F>(component_id: impl Into<String>, registration: F) -> Self
    where
        F: Fn(&FlowBus) -> LFResult<()> + Send + Sync + 'static,
    {
        Self {
            component_id: component_id.into(),
            registration: std::sync::Arc::new(registration),
        }
    }

    /// 返回组件 id，用于结构化诊断。
    #[must_use]
    pub fn component_id(&self) -> &str {
        &self.component_id
    }

    /// 向给定 FlowBus 提交注册。
    pub fn apply(&self, flow_bus: &FlowBus) -> LFResult<()> {
        (self.registration)(flow_bus)
    }
}
