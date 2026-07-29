//! 对应 Java: com.yomahub.liteflow.script.validator.ScriptValidator

use std::collections::HashMap;

use crate::common::entity::ValidationResp;
use crate::enums::ScriptTypeEnum;
use crate::exception::{LFResult, LiteflowError};
use crate::script::exception::ScriptSpiException;
use crate::script::{RhaiScriptExecutor, ScriptExecutorFactory, ScriptKind};

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

    /// 使用 Java `ScriptTypeEnum` 校验指定脚本。
    ///
    /// `script` 是待编译脚本文本，`script_type` 是 Java 对等脚本类型；返回是否
    /// 通过校验。对应 Java: `ScriptValidator#validate(String, ScriptTypeEnum)`。
    #[must_use]
    pub fn validate_with_script_type(script: &str, script_type: ScriptTypeEnum) -> bool {
        Self::validate_with_ex_for_script_type(script, script_type).is_success()
    }

    /// 使用 Java `ScriptTypeEnum` 校验脚本并保留失败原因。
    ///
    /// `script` 是待编译脚本文本，`script_type` 决定执行器；返回携带异常的
    /// `ValidationResp`。对应 Java:
    /// `ScriptValidator#validateWithEx(String, ScriptTypeEnum)`。
    #[must_use]
    pub fn validate_with_ex_for_script_type(
        script: &str,
        script_type: ScriptTypeEnum,
    ) -> ValidationResp {
        Self::validate_for_language_with_ex(script_type.get_display_name(), script)
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

    /// 按 Java Map 重载语义批量校验脚本。
    ///
    /// `scripts` 的每一项依次包含脚本类型与脚本文本；任意一项失败立即返回
    /// `false`，全部成功返回 `true`。对应 Java:
    /// `ScriptValidator#validate(Map<ScriptTypeEnum, String>)`。
    #[must_use]
    pub fn validate_scripts(
        scripts: impl IntoIterator<Item = (ScriptTypeEnum, impl AsRef<str>)>,
    ) -> bool {
        scripts.into_iter().all(|(script_type, script)| {
            Self::validate_with_script_type(script.as_ref(), script_type)
        })
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
