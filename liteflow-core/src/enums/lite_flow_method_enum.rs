//! 声明式组件可编排方法的元数据。
//!
//! 对应 `@LiteflowMethod` 与 `@LiteflowCmpDefine` 处理流程。
//! isMainMethod=true 的方法对应 EL 中的主逻辑（processXxx），
//! false 的为辅助方法（isAccess/isContinueOnError/beforeProcess 等）。

/// 标识声明式组件中可被 LiteFlow 代理的方法。
///
/// Rust 枚举保持方法名与主方法标志的固定映射，不提供 Java 的可变 setter，
/// 避免注册后破坏代理分派不变量。对应 Java:
/// `com.yomahub.liteflow.enums.LiteFlowMethodEnum`。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LiteFlowMethodEnum {
    Process,
    ProcessSwitch,
    ProcessBoolean,
    ProcessFor,
    ProcessIterator,
    IsAccess,
    IsEnd,
    IsContinueOnError,
    GetNodeExecutorClass,
    OnSuccess,
    OnError,
    BeforeProcess,
    AfterProcess,
    GetDisplayName,
    Rollback,
}

impl LiteFlowMethodEnum {
    /// 返回声明式组件中的 Java 方法名。
    ///
    /// 返回值直接对应 Java: `LiteFlowMethodEnum#getMethodName`。
    #[must_use]
    pub fn get_method_name(&self) -> &'static str {
        match self {
            Self::Process => "process",
            Self::ProcessSwitch => "processSwitch",
            Self::ProcessBoolean => "processBoolean",
            Self::ProcessFor => "processFor",
            Self::ProcessIterator => "processIterator",
            Self::IsAccess => "isAccess",
            Self::IsEnd => "isEnd",
            Self::IsContinueOnError => "isContinueOnError",
            Self::GetNodeExecutorClass => "getNodeExecutorClass",
            Self::OnSuccess => "onSuccess",
            Self::OnError => "onError",
            Self::BeforeProcess => "beforeProcess",
            Self::AfterProcess => "afterProcess",
            Self::GetDisplayName => "getDisplayName",
            Self::Rollback => "rollback",
        }
    }
    /// 返回当前方法是否为组件主逻辑方法。
    ///
    /// `process`、`processSwitch`、`processBoolean`、`processFor` 和
    /// `processIterator` 返回 true。对应 Java:
    /// `LiteFlowMethodEnum#isMainMethod`。
    #[must_use]
    pub fn is_main_method(&self) -> bool {
        matches!(
            self,
            Self::Process
                | Self::ProcessSwitch
                | Self::ProcessBoolean
                | Self::ProcessFor
                | Self::ProcessIterator
        )
    }
    /// 按 Java 方法名反查枚举。
    ///
    /// 参数 `name` 是声明式方法名；无法识别时返回 `None`。该 Rust 便利入口服务
    /// 于 Java 按 `methodName` 匹配的声明式方法解析。
    #[must_use]
    pub fn get_enum_by_method_name(name: &str) -> Option<Self> {
        [
            Self::Process,
            Self::ProcessSwitch,
            Self::ProcessBoolean,
            Self::ProcessFor,
            Self::ProcessIterator,
            Self::IsAccess,
            Self::IsEnd,
            Self::IsContinueOnError,
            Self::GetNodeExecutorClass,
            Self::OnSuccess,
            Self::OnError,
            Self::BeforeProcess,
            Self::AfterProcess,
            Self::GetDisplayName,
            Self::Rollback,
        ]
        .into_iter()
        .find(|e| e.get_method_name() == name)
    }
}
