use crate::enums::ConditionTypeEnum;
use serde::{Deserialize, Serialize};

/// 构建 Chain 的中间属性。
///
/// 字段保持可空语义，以 `Option` 对齐 Java bean 尚未赋值时的 null；
/// serde 负责 Jackson 在 JSON/YAML 配置中的职责。
/// 对应 Java: `com.yomahub.liteflow.builder.prop.ChainPropBean`。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ChainPropBean {
    /// 执行规则。
    pub cond_value_str: Option<String>,
    /// 分组。
    pub group: Option<String>,
    /// 是否抛出异常。
    pub error_resume: Option<String>,
    /// 满足任意条件后完成。
    pub any: Option<String>,
    /// 指定线程池。
    pub thread_executor_class: Option<String>,
    /// Chain 条件类型。
    pub condition_type: Option<ConditionTypeEnum>,
}

impl ChainPropBean {
    /// 返回执行规则。对应 Java: `ChainPropBean#getCondValueStr`。
    #[must_use]
    pub fn get_cond_value_str(&self) -> Option<&str> {
        self.cond_value_str.as_deref()
    }

    /// 返回执行规则。
    ///
    /// Rust 历史便捷入口；Java 对等名称请使用 `get_cond_value_str`。
    #[must_use]
    pub fn cond_value_str(&self) -> Option<&str> {
        self.get_cond_value_str()
    }

    /// 设置执行规则。对应 Java: `ChainPropBean#setCondValueStr`。
    pub fn set_cond_value_str(mut self, cond_value_str: impl Into<String>) -> Self {
        self.cond_value_str = Some(cond_value_str.into());
        self
    }

    /// 返回分组。对应 Java: `ChainPropBean#getGroup`。
    #[must_use]
    pub fn get_group(&self) -> Option<&str> {
        self.group.as_deref()
    }

    /// 返回分组。
    ///
    /// Rust 历史便捷入口；Java 对等名称请使用 `get_group`。
    #[must_use]
    pub fn group(&self) -> Option<&str> {
        self.get_group()
    }

    /// 设置分组。对应 Java: `ChainPropBean#setGroup`。
    pub fn set_group(mut self, group: impl Into<String>) -> Self {
        self.group = Some(group.into());
        self
    }

    /// 返回错误恢复配置。对应 Java: `ChainPropBean#getErrorResume`。
    #[must_use]
    pub fn get_error_resume(&self) -> Option<&str> {
        self.error_resume.as_deref()
    }

    /// 返回错误恢复配置。
    ///
    /// Rust 历史便捷入口；Java 对等名称请使用 `get_error_resume`。
    #[must_use]
    pub fn error_resume(&self) -> Option<&str> {
        self.get_error_resume()
    }

    /// 设置错误恢复配置。对应 Java: `ChainPropBean#setErrorResume`。
    pub fn set_error_resume(mut self, error_resume: impl Into<String>) -> Self {
        self.error_resume = Some(error_resume.into());
        self
    }

    /// 返回 any 配置。对应 Java: `ChainPropBean#getAny`。
    #[must_use]
    pub fn get_any(&self) -> Option<&str> {
        self.any.as_deref()
    }

    /// 返回 any 配置。
    ///
    /// Rust 历史便捷入口；Java 对等名称请使用 `get_any`。
    #[must_use]
    pub fn any(&self) -> Option<&str> {
        self.get_any()
    }

    /// 设置 any 配置。对应 Java: `ChainPropBean#setAny`。
    pub fn set_any(mut self, any: impl Into<String>) -> Self {
        self.any = Some(any.into());
        self
    }

    /// 返回线程池类名。对应 Java: `ChainPropBean#getThreadExecutorClass`。
    #[must_use]
    pub fn get_thread_executor_class(&self) -> Option<&str> {
        self.thread_executor_class.as_deref()
    }

    /// 返回线程池类名。
    ///
    /// Rust 历史便捷入口；Java 对等名称请使用 `get_thread_executor_class`。
    #[must_use]
    pub fn thread_executor_class(&self) -> Option<&str> {
        self.get_thread_executor_class()
    }

    /// 设置线程池类名。对应 Java: `ChainPropBean#setThreadExecutorClass`。
    pub fn set_thread_executor_class(mut self, thread_executor_class: impl Into<String>) -> Self {
        self.thread_executor_class = Some(thread_executor_class.into());
        self
    }

    /// 返回条件类型。对应 Java: `ChainPropBean#getConditionType`。
    #[must_use]
    pub fn get_condition_type(&self) -> Option<ConditionTypeEnum> {
        self.condition_type
    }

    /// 返回条件类型。
    ///
    /// Rust 历史便捷入口；Java 对等名称请使用 `get_condition_type`。
    #[must_use]
    pub fn condition_type(&self) -> Option<ConditionTypeEnum> {
        self.get_condition_type()
    }

    /// 设置条件类型。对应 Java: `ChainPropBean#setConditionType`。
    pub fn set_condition_type(mut self, condition_type: ConditionTypeEnum) -> Self {
        self.condition_type = Some(condition_type);
        self
    }
}
