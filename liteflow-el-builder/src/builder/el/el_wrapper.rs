pub(crate) use super::ELWrapperData;
pub use super::{
    BoxedELWrapper, ELBuilderError, ELBuilderResult, IntoELWrapper, RenderMode, WrapperKind,
};

/// ELWrapper 是所有组件的抽象父协议。
///
/// 定义所有 EL 表达式共有的 `tag`、`id`、`data`、`bind`、
/// `maxWaitSeconds`、`retry` 能力和子表达式渲染协议。
/// 对应 Java: `com.yomahub.liteflow.builder.el.ELWrapper`。
pub trait ELWrapper: Send + Sync {
    /// 返回包装器类别，供 `ELBus` 执行与 Java `instanceof` 等价的校验。
    fn wrapper_kind(&self) -> WrapperKind;

    /// 渲染当前包装器。
    fn render_el(
        &self,
        depth: Option<usize>,
        param_context: &mut String,
        mode: RenderMode,
    ) -> ELBuilderResult<String>;

    /// 非格式化输出 Java 风格完整 EL 语句。
    ///
    /// 返回值包含 data 参数声明和末尾分号，对应 Java: `ELWrapper#toEL()`。
    fn to_el(&self) -> ELBuilderResult<String> {
        self.to_el_with_format(false)
    }

    /// 按需格式化输出 Java 风格完整 EL 语句。
    ///
    /// # 参数
    /// - `format`: `true` 时输出带缩进的树形结构。
    ///
    /// # 返回
    /// 包含参数声明和末尾分号的 EL 语句。
    /// 对应 Java: `ELWrapper#toEL(boolean)`。
    fn to_el_with_format(&self, format: bool) -> ELBuilderResult<String> {
        let mut params = String::new();
        let expression =
            self.render_el(format.then_some(0), &mut params, RenderMode::JavaStatement)?;
        Ok(format!("{params}{expression};"))
    }

    /// 输出可直接交给 `liteflow_core::parse_el` 的 Rust 运行时表达式。
    ///
    /// 与 Java 完整语句相比，该形式不含参数声明和末尾分号，并把 data JSON
    /// 直接内联；retry 的 Java 异常类名由 Rust 组件的错误谓词负责，不写入语法。
    fn to_expression(&self) -> ELBuilderResult<String> {
        self.render_el(None, &mut String::new(), RenderMode::RuntimeExpression)
    }
}

impl ELWrapper for BoxedELWrapper {
    fn wrapper_kind(&self) -> WrapperKind {
        (**self).wrapper_kind()
    }

    fn render_el(
        &self,
        depth: Option<usize>,
        param_context: &mut String,
        mode: RenderMode,
    ) -> ELBuilderResult<String> {
        (**self).render_el(depth, param_context, mode)
    }
}

pub(crate) fn render_call(
    name: &str,
    children: &[BoxedELWrapper],
    depth: Option<usize>,
    param_context: &mut String,
    mode: RenderMode,
) -> ELBuilderResult<String> {
    let child_depth = depth.map(|value| value + 1);
    let mut output = tabs(depth);
    output.push_str(name);
    output.push('(');
    newline(&mut output, depth);
    for (index, child) in children.iter().enumerate() {
        if index > 0 {
            output.push(',');
            newline(&mut output, depth);
        }
        output.push_str(&child.render_el(child_depth, param_context, mode)?);
    }
    newline(&mut output, depth);
    output.push_str(&tabs(depth));
    output.push(')');
    Ok(output)
}

pub(crate) fn tabs(depth: Option<usize>) -> String {
    depth.map(|value| "\t".repeat(value)).unwrap_or_default()
}

pub(crate) fn newline(output: &mut String, depth: Option<usize>) {
    if depth.is_some() {
        output.push('\n');
    }
}

pub(crate) fn escape_el_string(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

macro_rules! impl_common_fluent {
    ($type_name:ty) => {
        /// 设置组件标记内容。对应 Java: `ELWrapper#tag`。
        pub fn tag(mut self, tag: impl Into<String>) -> Self {
            self.properties.set_tag(tag);
            self
        }

        /// 设置组件实例 id。对应 Java: `ELWrapper#id`。
        pub fn id(mut self, id: impl Into<String>) -> Self {
            self.properties.set_id(id);
            self
        }

        /// 使用 serde 序列化对象并设置表达式 data。
        /// 对应 Java: `ELWrapper#data(String, Object)`。
        pub fn data<T: serde::Serialize>(
            mut self,
            data_name: impl Into<String>,
            value: &T,
        ) -> crate::builder::el::el_wrapper::ELBuilderResult<Self> {
            self.properties.set_data(data_name, value)?;
            Ok(self)
        }

        /// 使用已有 JSON 字符串设置表达式 data。
        /// 对应 Java: `ELWrapper#data(String, String)`。
        pub fn data_json(mut self, data_name: impl Into<String>, json: impl Into<String>) -> Self {
            self.properties.set_data_json(data_name, json);
            self
        }

        /// 设置表达式 bind 键值。对应 Java: `ELWrapper#bind`。
        pub fn bind(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
            self.properties.bind(key, value);
            self
        }

        /// 设置最长等待秒数。对应 Java: `ELWrapper#maxWaitSeconds`。
        pub fn max_wait_seconds(mut self, seconds: u64) -> Self {
            self.properties.set_max_wait_seconds(seconds);
            self
        }

        /// 设置最大重试次数。对应 Java: `ELWrapper#retry(int)`。
        pub fn retry(mut self, count: u32) -> Self {
            self.properties
                .set_retry(crate::builder::el::vo::RetryELVo::new(count));
            self
        }

        /// 设置最大重试次数和 Java 异常类型名。
        /// 对应 Java: `ELWrapper#retry(int, String...)`。
        pub fn retry_for<I, S>(mut self, count: u32, exceptions: I) -> Self
        where
            I: IntoIterator<Item = S>,
            S: Into<String>,
        {
            self.properties
                .set_retry(crate::builder::el::vo::RetryELVo::with_exceptions(
                    count, exceptions,
                ));
            self
        }
    };
}

pub(crate) use impl_common_fluent;
