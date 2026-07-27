use serde::{Deserialize, Serialize};

/// 构建 Node 的中间属性。
///
/// serde 同时接受 Java 规则中的 `class`/`value` 与对象字段名
/// `clazz`/`script`，用于 JSON/YAML 统一反序列化。
/// 对应 Java: `com.yomahub.liteflow.builder.prop.NodePropBean`。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct NodePropBean {
    /// 节点 id。
    pub id: Option<String>,
    /// 节点名称。
    pub name: Option<String>,
    /// Java 类名；Rust 端作为注册键/诊断元数据保留。
    #[serde(rename = "class", alias = "clazz")]
    pub clazz: Option<String>,
    /// 脚本内容。
    #[serde(alias = "value")]
    pub script: Option<String>,
    /// 节点类型 code。
    #[serde(rename = "type")]
    pub node_type: Option<String>,
    /// 脚本文件位置。
    pub file: Option<String>,
    /// 脚本语言。
    pub language: Option<String>,
}

impl NodePropBean {
    /// 返回节点 id。对应 Java: `NodePropBean#getId`。
    #[must_use]
    pub fn get_id(&self) -> Option<&str> {
        self.id.as_deref()
    }

    /// 返回节点 id。
    ///
    /// Rust 历史便捷入口；Java 对等名称请使用 `get_id`。
    #[must_use]
    pub fn id(&self) -> Option<&str> {
        self.get_id()
    }

    /// 设置节点 id。对应 Java: `NodePropBean#setId`。
    pub fn set_id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// 返回节点名称。对应 Java: `NodePropBean#getName`。
    #[must_use]
    pub fn get_name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// 返回节点名称。
    ///
    /// Rust 历史便捷入口；Java 对等名称请使用 `get_name`。
    #[must_use]
    pub fn name(&self) -> Option<&str> {
        self.get_name()
    }

    /// 设置节点名称。对应 Java: `NodePropBean#setName`。
    pub fn set_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// 返回类名。对应 Java: `NodePropBean#getClazz`。
    #[must_use]
    pub fn get_clazz(&self) -> Option<&str> {
        self.clazz.as_deref()
    }

    /// 返回类名。
    ///
    /// Rust 历史便捷入口；Java 对等名称请使用 `get_clazz`。
    #[must_use]
    pub fn clazz(&self) -> Option<&str> {
        self.get_clazz()
    }

    /// 设置类名。对应 Java: `NodePropBean#setClazz`。
    pub fn set_clazz(mut self, clazz: impl Into<String>) -> Self {
        self.clazz = Some(clazz.into());
        self
    }

    /// 返回脚本。对应 Java: `NodePropBean#getScript`。
    #[must_use]
    pub fn get_script(&self) -> Option<&str> {
        self.script.as_deref()
    }

    /// 返回脚本。
    ///
    /// Rust 历史便捷入口；Java 对等名称请使用 `get_script`。
    #[must_use]
    pub fn script(&self) -> Option<&str> {
        self.get_script()
    }

    /// 设置脚本。对应 Java: `NodePropBean#setScript`。
    pub fn set_script(mut self, script: impl Into<String>) -> Self {
        self.script = Some(script.into());
        self
    }

    /// 返回类型 code。对应 Java: `NodePropBean#getType`。
    #[must_use]
    pub fn get_type(&self) -> Option<&str> {
        self.node_type.as_deref()
    }

    /// 返回类型 code。
    ///
    /// Rust 历史便捷入口；Java 对等名称请使用 `get_type`。
    #[must_use]
    pub fn node_type(&self) -> Option<&str> {
        self.get_type()
    }

    /// 设置类型 code。对应 Java: `NodePropBean#setType`。
    pub fn set_type(mut self, node_type: impl Into<String>) -> Self {
        self.node_type = Some(node_type.into());
        self
    }

    /// 返回脚本文件路径。对应 Java: `NodePropBean#getFile`。
    #[must_use]
    pub fn get_file(&self) -> Option<&str> {
        self.file.as_deref()
    }

    /// 返回脚本文件路径。
    ///
    /// Rust 历史便捷入口；Java 对等名称请使用 `get_file`。
    #[must_use]
    pub fn file(&self) -> Option<&str> {
        self.get_file()
    }

    /// 设置脚本文件路径。对应 Java: `NodePropBean#setFile`。
    pub fn set_file(mut self, file: impl Into<String>) -> Self {
        self.file = Some(file.into());
        self
    }

    /// 返回脚本语言。对应 Java: `NodePropBean#getLanguage`。
    #[must_use]
    pub fn get_language(&self) -> Option<&str> {
        self.language.as_deref()
    }

    /// 返回脚本语言。
    ///
    /// Rust 历史便捷入口；Java 对等名称请使用 `get_language`。
    #[must_use]
    pub fn language(&self) -> Option<&str> {
        self.get_language()
    }

    /// 设置脚本语言。对应 Java: `NodePropBean#setLanguage`。
    pub fn set_language(mut self, language: impl Into<String>) -> Self {
        self.language = Some(language.into());
        self
    }
}
