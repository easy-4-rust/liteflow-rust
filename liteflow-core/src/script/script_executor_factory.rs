//! 对应 Java 类：`com.yomahub.liteflow.script.ScriptExecutorFactory`。
//!
//! Java 用 ServiceLoader 发现脚本插件；Rust 插件 crate 通过显式注册构建函数，
//! 避免 core 反向依赖各语言实现。

use std::sync::{Arc, OnceLock};

use dashmap::DashMap;

use crate::core::NodeComponent;
use crate::exception::{LFResult, LiteflowError};

use super::exception::ScriptSpiException;
use super::{ScriptComponentBuilder, ScriptKind};

fn registry() -> &'static DashMap<String, ScriptComponentBuilder> {
    static REGISTRY: OnceLock<DashMap<String, ScriptComponentBuilder>> = OnceLock::new();
    REGISTRY.get_or_init(DashMap::new)
}

/// 脚本执行器工厂。
///
/// 对应 Java `ScriptExecutorFactory#scriptExecutorMap`。注册按 language 覆盖，
/// 便于测试或应用替换实现。
pub struct ScriptExecutorFactory;

impl ScriptExecutorFactory {
    /// 返回进程级脚本执行器工厂。
    ///
    /// 工厂本身无可变字段，语言注册表由线程安全全局容器持有。对应 Java:
    /// `ScriptExecutorFactory#loadInstance`。
    #[must_use]
    pub fn load_instance() -> &'static Self {
        static INSTANCE: ScriptExecutorFactory = ScriptExecutorFactory;
        &INSTANCE
    }

    /// 注册语言构建器。对应 Java ServiceLoader 发现并缓存 ScriptExecutor。
    pub fn register(language: impl Into<String>, builder: ScriptComponentBuilder) -> LFResult<()> {
        let language = language.into();
        if language.trim().is_empty() {
            return Err(LiteflowError::Script {
                node: String::new(),
                msg: "script language cannot be blank".to_string(),
            });
        }
        registry().insert(language, builder);
        Ok(())
    }

    /// 删除指定语言实现。
    pub fn unregister(language: &str) {
        registry().remove(language);
    }

    /// 是否已注册指定语言。
    pub fn contains(language: &str) -> bool {
        registry().contains_key(language)
    }

    /// 构建插件脚本组件。对应 Java `getScriptExecutor(language)` 后 `load`。
    pub fn build(
        language: &str,
        node_id: &str,
        kind: ScriptKind,
        script: &str,
    ) -> LFResult<Arc<dyn NodeComponent>> {
        let builder = Self::load_instance().get_script_executor(language)?;
        builder(node_id, kind, script)
    }

    /// 返回指定语言已经注册的真实脚本组件构建器。
    ///
    /// Java 返回缓存的 `ScriptExecutor`；Rust 插件以构建器作为执行器注册句柄，
    /// 它会创建并加载真实 `ScriptExecutorComponent`。空语言没有隐式默认实现，
    /// 未注册时返回 `ScriptSpiException`。对应 Java:
    /// `ScriptExecutorFactory#getScriptExecutor`。
    pub fn get_script_executor(&self, language: &str) -> LFResult<ScriptComponentBuilder> {
        let language = language.trim();
        registry()
            .get(language)
            .map(|entry| *entry)
            .ok_or_else(|| {
                ScriptSpiException::new(format!(
                    "unsupported script language: {language}; register it through liteflow-script-plugin"
                ))
                .into()
            })
    }

    /// 清空插件缓存。对应 Java `cleanScriptCache`。
    pub fn clean() {
        registry().clear();
    }

    /// 清空全部脚本执行器注册缓存。
    ///
    /// 清理后插件需要重新注册才能构建脚本组件。对应 Java:
    /// `ScriptExecutorFactory#cleanScriptCache`。
    pub fn clean_script_cache(&self) {
        Self::clean();
    }

    /// 已注册语言列表。
    pub fn languages() -> Vec<String> {
        let mut languages = registry()
            .iter()
            .map(|entry| entry.key().clone())
            .collect::<Vec<_>>();
        languages.sort();
        languages
    }
}
