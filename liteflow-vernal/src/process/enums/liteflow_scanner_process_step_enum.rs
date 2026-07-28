/// LiteFlow 容器 Bean 扫描步骤及其固定优先级。
///
/// 顺序与 Java 枚举完全一致；数据库连接步骤由独立数据源模块消费，因此工厂
/// 不注册该占位优先级。对应 Java:
/// `com.yomahub.liteflow.spring.process.enums.LiteflowScannerProcessStepEnum`。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LiteflowScannerProcessStepEnum {
    /// 声明式包装对象，优先级 1。
    DeclWarpBean,
    /// 普通节点组件，优先级 2。
    NodeCmpBean,
    /// 全局节点切面，优先级 3。
    CmpAroundAspectBean,
    /// `@ScriptBean` 对象，优先级 4。
    ScriptBean,
    /// `@ScriptMethod` 方法组，优先级 5。
    ScriptMethodBean,
    /// 数据库连接对象，优先级 6。
    DataBaseConnectBean,
    /// 生命周期对象，优先级 7。
    LifeCycleBean,
}

impl LiteflowScannerProcessStepEnum {
    /// 返回扫描步骤优先级。
    ///
    /// # 返回
    /// Java 枚举构造参数中的整数优先级。对应 Java:
    /// `LiteflowScannerProcessStepEnum#getPriority`。
    #[must_use]
    pub const fn priority(self) -> u8 {
        match self {
            Self::DeclWarpBean => 1,
            Self::NodeCmpBean => 2,
            Self::CmpAroundAspectBean => 3,
            Self::ScriptBean => 4,
            Self::ScriptMethodBean => 5,
            Self::DataBaseConnectBean => 6,
            Self::LifeCycleBean => 7,
        }
    }

    /// 返回扫描步骤中文说明。
    ///
    /// # 返回
    /// 与 Java 枚举 `desc` 字段等价的稳定诊断文本。对应 Java:
    /// `LiteflowScannerProcessStepEnum#getDesc`。
    #[must_use]
    pub const fn description(self) -> &'static str {
        match self {
            Self::DeclWarpBean => "声明式组件",
            Self::NodeCmpBean => "普通组件",
            Self::CmpAroundAspectBean => "组件切面",
            Self::ScriptBean => "脚本Bean",
            Self::ScriptMethodBean => "脚本方法",
            Self::DataBaseConnectBean => "数据库连接",
            Self::LifeCycleBean => "生命周期组件",
        }
    }
}
