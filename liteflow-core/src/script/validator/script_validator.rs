//! 对应 Java: com.yomahub.liteflow.script.validator.ScriptValidator

use std::collections::HashMap;

use crate::common::entity::ValidationResp;
use crate::exception::{LFResult, LiteflowError};
use crate::script::exception::ScriptSpiException;
use crate::script::script_executor::RhaiScriptExecutor;
use crate::script::{ScriptExecutorFactory, ScriptKind};

/// 按已发现语言实现校验单个或批量脚本。
pub struct ScriptValidator;

impl ScriptValidator {
    fn languages() -> Vec<String> {
        let mut languages = ScriptExecutorFactory::languages();
        if !languages.iter().any(|language| language == "rhai") {
            languages.push("rhai".to_string());
        }
        languages.sort();
        languages
    }

    /// 在仅存在一种语言时校验脚本；多语言环境必须显式指定语言。
    pub fn validate(script: &str) -> bool {
        Self::validate_with_ex(script).is_success()
    }

    /// 返回保留失败原因的单语言校验结果。
    #[must_use]
    pub fn validate_with_ex(script: &str) -> ValidationResp {
        let languages = Self::languages();
        if languages.len() != 1 {
            return ValidationResp::fail(
                ScriptSpiException::new(format!(
                    "found {} script languages; language must be specified",
                    languages.len()
                ))
                .into(),
            );
        }
        Self::validate_for_language_with_ex(&languages[0], script)
    }

    /// 使用指定语言校验脚本。
    pub fn validate_for_language(language: &str, script: &str) -> bool {
        Self::validate_for_language_with_ex(language, script).is_success()
    }

    /// 使用指定语言校验脚本并保留异常。
    #[must_use]
    pub fn validate_for_language_with_ex(language: &str, script: &str) -> ValidationResp {
        if language == "rhai" {
            return RhaiScriptExecutor::new().validate_with_ex(script);
        }
        match ScriptExecutorFactory::build(language, "__validate__", ScriptKind::Common, script) {
            Ok(_) => ValidationResp::success(),
            Err(error) => ValidationResp::fail(error),
        }
    }

    /// 批量校验 language→script，并返回每种语言的独立结果。
    #[must_use]
    pub fn validate_batch(
        scripts: impl IntoIterator<Item = (impl Into<String>, impl Into<String>)>,
    ) -> HashMap<String, ValidationResp> {
        scripts
            .into_iter()
            .map(|(language, script)| {
                let language = language.into();
                let result = Self::validate_for_language_with_ex(&language, &script.into());
                (language, result)
            })
            .collect()
    }

    /// 严格校验入口，失败时直接返回异常，便于 Rust 调用方使用 `?`。
    pub fn ensure_valid(language: &str, script: &str) -> LFResult<()> {
        let response = Self::validate_for_language_with_ex(language, script);
        if response.is_success() {
            Ok(())
        } else {
            Err(response
                .cause()
                .cloned()
                .unwrap_or_else(|| LiteflowError::Script {
                    node: String::new(),
                    msg: "script validation failed".to_string(),
                }))
        }
    }
}
