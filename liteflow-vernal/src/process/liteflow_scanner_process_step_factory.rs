use std::sync::Arc;

use crate::process::LiteflowScannerProcessStep;
use crate::process::r#impl::{
    CmpAroundAspectBeanProcess, DeclWarpBeanProcess, LifeCycleBeanProcess, NodeCmpBeanProcess,
    ScriptBeanProcess, ScriptMethodBeanProcess,
};

/// 构造并排序 LiteFlow 默认扫描步骤。
///
/// Java 使用静态列表；Rust 工厂作为 Vernal 上下文单例，避免多次启动时重复追加
/// 同一处理器。对应 Java:
/// `com.yomahub.liteflow.spring.process.LiteflowScannerProcessStepFactory`。
pub struct LiteflowScannerProcessStepFactory {
    process_steps: Vec<Arc<dyn LiteflowScannerProcessStep>>,
}

impl LiteflowScannerProcessStepFactory {
    /// 创建包含六个 Spring 默认处理器的工厂。
    ///
    /// # 返回
    /// 按枚举优先级升序排列的不可变步骤集合。对应 Java:
    /// `LiteflowScannerProcessStepFactory#LiteflowScannerProcessStepFactory`。
    #[must_use]
    pub fn new() -> Self {
        let mut process_steps: Vec<Arc<dyn LiteflowScannerProcessStep>> = vec![
            Arc::new(DeclWarpBeanProcess::new()),
            Arc::new(NodeCmpBeanProcess::new()),
            Arc::new(CmpAroundAspectBeanProcess::new()),
            Arc::new(ScriptBeanProcess::new()),
            Arc::new(ScriptMethodBeanProcess::new()),
            Arc::new(LifeCycleBeanProcess::new()),
        ];
        process_steps.sort_by_key(|step| step.step_type().priority());
        Self { process_steps }
    }

    /// 返回有序处理步骤。
    ///
    /// # 返回
    /// 只读切片，调用方不能改变工厂顺序。对应 Java:
    /// `LiteflowScannerProcessStepFactory#getProcessSteps`。
    #[must_use]
    pub fn get_process_steps(&self) -> &[Arc<dyn LiteflowScannerProcessStep>] {
        &self.process_steps
    }
}

impl Default for LiteflowScannerProcessStepFactory {
    fn default() -> Self {
        Self::new()
    }
}
