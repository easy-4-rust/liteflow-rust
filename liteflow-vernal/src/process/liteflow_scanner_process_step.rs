use liteflow_core::LFResult;

use crate::LiteflowComponentRegistration;
use crate::process::context::LiteflowScannerProcessStepContext;
use crate::process::enums::LiteflowScannerProcessStepEnum;

/// LiteFlow 容器 Bean 扫描步骤协议。
///
/// 扫描器按优先级调用 `filter`，仅执行首个匹配步骤的后处理，保持 Java
/// BeanPostProcessor 链的短路语义。对应 Java:
/// `com.yomahub.liteflow.spring.process.LiteflowScannerProcessStep`。
pub trait LiteflowScannerProcessStep: Send + Sync {
    /// 判断当前 Bean 是否由本步骤处理。
    ///
    /// # 参数
    /// - `context`：包含当前注册定义与扫描期中间结果的上下文。
    ///
    /// # 返回
    /// 匹配返回 `true`。对应 Java: `LiteflowScannerProcessStep#filter`。
    fn filter(&self, context: &mut LiteflowScannerProcessStepContext<'_>) -> bool;

    /// 对匹配 Bean 执行初始化后处理。
    ///
    /// # 参数
    /// - `context`：已通过本步骤过滤的扫描上下文。
    ///
    /// # 返回
    /// Java 返回的 Bean 等价注册定义；失败返回真实 LiteFlow 错误。对应 Java:
    /// `LiteflowScannerProcessStep#postProcessAfterInitialization`。
    fn post_process_after_initialization(
        &self,
        context: &mut LiteflowScannerProcessStepContext<'_>,
    ) -> LFResult<LiteflowComponentRegistration>;

    /// 返回步骤类型和排序优先级。
    ///
    /// # 返回
    /// 对应 Java 枚举项。对应 Java: `LiteflowScannerProcessStep#type`。
    fn step_type(&self) -> LiteflowScannerProcessStepEnum;
}
