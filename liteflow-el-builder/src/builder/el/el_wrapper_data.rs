//! Java `ELWrapper` 公共字段载体。

use std::collections::BTreeMap;

use serde::Serialize;

use super::el_wrapper::escape_el_string;
use super::vo::RetryELVo;
use super::{ELBuilderError, ELBuilderResult, RenderMode};

/// Java `ELWrapper` 的公共字段载体。
#[derive(Debug, Clone, Default)]
pub(crate) struct ELWrapperData {
    tag: Option<String>,
    id: Option<String>,
    data_name: Option<String>,
    data: Option<String>,
    bind_data: BTreeMap<String, String>,
    max_wait_seconds: Option<u64>,
    retry: Option<RetryELVo>,
}

impl ELWrapperData {
    pub(crate) fn set_tag(&mut self, tag: impl Into<String>) {
        self.tag = Some(tag.into());
    }

    pub(crate) fn set_id(&mut self, id: impl Into<String>) {
        self.id = Some(id.into());
    }

    pub(crate) fn set_data_json(&mut self, data_name: impl Into<String>, json: impl Into<String>) {
        self.data_name = Some(data_name.into());
        self.data = Some(json.into());
    }

    pub(crate) fn set_data<T: Serialize>(
        &mut self,
        data_name: impl Into<String>,
        value: &T,
    ) -> ELBuilderResult<()> {
        let json = serde_json::to_string(value)
            .map_err(|error| ELBuilderError::DataSerialization(error.to_string()))?;
        self.set_data_json(data_name, json);
        Ok(())
    }

    pub(crate) fn bind(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.bind_data.insert(key.into(), value.into());
    }

    pub(crate) fn set_max_wait_seconds(&mut self, seconds: u64) {
        self.max_wait_seconds = Some(seconds);
    }

    pub(crate) fn set_retry(&mut self, retry: RetryELVo) {
        self.retry = Some(retry);
    }

    pub(crate) fn append_properties(
        &self,
        el_context: &mut String,
        param_context: &mut String,
        mode: RenderMode,
    ) {
        if let Some(id) = &self.id {
            el_context.push_str(&format!(".id(\"{}\")", escape_el_string(id)));
        }
        if let Some(tag) = &self.tag {
            el_context.push_str(&format!(".tag(\"{}\")", escape_el_string(tag)));
        }
        if let (Some(data_name), Some(data)) = (&self.data_name, &self.data) {
            let escaped = escape_el_string(data);
            match mode {
                RenderMode::JavaStatement => {
                    el_context.push_str(&format!(".data({data_name})"));
                    param_context.push_str(&format!("{data_name} = \"{escaped}\";\n"));
                }
                RenderMode::RuntimeExpression => {
                    el_context.push_str(&format!(".data(\"{escaped}\")"));
                }
            }
        }
        for (key, value) in &self.bind_data {
            el_context.push_str(&format!(
                ".bind(\"{}\", \"{}\")",
                escape_el_string(key),
                escape_el_string(value)
            ));
        }
        if let Some(seconds) = self.max_wait_seconds {
            el_context.push_str(&format!(".maxWaitSeconds({seconds})"));
        }
        if let Some(retry) = &self.retry {
            let text = match mode {
                RenderMode::JavaStatement => retry.to_string(),
                RenderMode::RuntimeExpression => retry.runtime_text(),
            };
            el_context.push_str(&format!(".retry({text})"));
        }
    }

    pub(crate) fn append_id_and_tag(&self, el_context: &mut String) {
        if let Some(id) = &self.id {
            el_context.push_str(&format!(".id(\"{}\")", escape_el_string(id)));
        }
        if let Some(tag) = &self.tag {
            el_context.push_str(&format!(".tag(\"{}\")", escape_el_string(tag)));
        }
    }
}
