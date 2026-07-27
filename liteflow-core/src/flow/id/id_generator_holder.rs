//! 对应 Java 类：com.yomahub.liteflow.flow.id.IdGeneratorHolder
//!
//! Id 生成器帮助器。Java 版 init() 读取 LiteflowConfig.requestIdGeneratorClass：
//! 为空则用 DefaultRequestIdGenerator，否则反射实例化并经 ContextAware 注册。
//! Rust 侧与 spi holder 同模式：`OnceLock<RwLock<Option<Arc<dyn RequestIdGenerator>>>>`
//! 全局单例。首次生成 ID 时按 `LiteflowConfig` 懒初始化；register() 可显式覆盖
//! 当前生成器，register_named() 用于把 Rust 实现绑定到 Java 配置类名。

use std::collections::HashMap;
use std::sync::{Arc, OnceLock, RwLock};

use crate::exception::{LFResult, RequestIdGeneratorException};
use crate::property::LiteflowConfigGetter;

use super::default_request_id_generator::DefaultRequestIdGenerator;
use super::request_id_generator::RequestIdGenerator;

const DEFAULT_REQUEST_ID_GENERATOR_CLASS: &str =
    "com.yomahub.liteflow.flow.id.DefaultRequestIdGenerator";

static HOLDER: OnceLock<RwLock<Option<Arc<dyn RequestIdGenerator>>>> = OnceLock::new();
static INSTANCE: IdGeneratorHolder = IdGeneratorHolder;

fn cell() -> &'static RwLock<Option<Arc<dyn RequestIdGenerator>>> {
    HOLDER.get_or_init(|| RwLock::new(None))
}

fn named_generators() -> &'static RwLock<HashMap<String, Arc<dyn RequestIdGenerator>>> {
    static NAMED_GENERATORS: OnceLock<RwLock<HashMap<String, Arc<dyn RequestIdGenerator>>>> =
        OnceLock::new();
    NAMED_GENERATORS.get_or_init(|| RwLock::new(HashMap::new()))
}

/// 对应 IdGeneratorHolder
pub struct IdGeneratorHolder;

impl IdGeneratorHolder {
    /// 按当前 LiteFlow 配置初始化 Request ID 生成器。
    ///
    /// Java 通过反射和 Spring 容器按类名构造对象；Rust 先从显式注册表解析同名
    /// 生成器，默认 Java 类名直接映射到 `DefaultRequestIdGenerator`。找不到自定义
    /// 类名时返回 `RequestIdGeneratorException` 对应错误。
    ///
    /// # 返回
    /// 初始化成功返回 `Ok(())`；自定义类名未注册时返回错误。
    ///
    /// 对应 Java: `IdGeneratorHolder#init`。
    pub fn init() -> LFResult<()> {
        let class_name = LiteflowConfigGetter::get()
            .get_request_id_generator_class()
            .trim()
            .to_string();
        let generator: Arc<dyn RequestIdGenerator> =
            if class_name.is_empty() || class_name == DEFAULT_REQUEST_ID_GENERATOR_CLASS {
                Arc::new(DefaultRequestIdGenerator::new())
            } else {
                named_generators()
                    .read()
                    .unwrap()
                    .get(&class_name)
                    .cloned()
                    .ok_or_else(|| {
                        RequestIdGeneratorException::new(format!(
                            "request id generator class[{class_name}] is not registered"
                        ))
                    })?
            };
        Self::set_request_id_generator(generator);
        Ok(())
    }

    /// 返回进程级持有器实例。
    ///
    /// # 返回
    /// 与 Java 静态 `INSTANCE` 对应的唯一零状态门面对象。
    ///
    /// 对应 Java: `IdGeneratorHolder#getInstance`。
    #[must_use]
    pub fn get_instance() -> &'static Self {
        &INSTANCE
    }

    /// 返回当前生成器；尚未初始化时返回 `None`。
    ///
    /// # 返回
    /// 当前共享生成器的引用计数句柄。
    ///
    /// 对应 Java: `IdGeneratorHolder#getRequestIdGenerator`。
    #[must_use]
    pub fn get_request_id_generator() -> Option<Arc<dyn RequestIdGenerator>> {
        cell().read().unwrap().clone()
    }

    /// 替换当前 Request ID 生成器。
    ///
    /// 参数 `request_id_generator` 对应 Java 同名参数。对应 Java:
    /// `IdGeneratorHolder#setRequestIdGenerator`。
    pub fn set_request_id_generator(request_id_generator: Arc<dyn RequestIdGenerator>) {
        *cell().write().unwrap() = Some(request_id_generator);
    }

    /// 返回当前生成器，尚未初始化时按 LiteFlow 配置完成懒初始化。
    ///
    /// Java 的 `generate()` 会在生成器为空时调用 `init()`；这里保留相同顺序，
    /// 因而自定义 `request_id_generator_class` 不会被默认生成器静默覆盖。
    ///
    /// # 返回
    /// 当前共享生成器；配置的自定义类名未注册时，与 Java 未检查异常一样终止调用。
    ///
    /// 对应 Java: `IdGeneratorHolder#generate`、`IdGeneratorHolder#init`。
    pub fn load_generator() -> Arc<dyn RequestIdGenerator> {
        if let Some(x) = cell().read().unwrap().as_ref() {
            return x.clone();
        }
        Self::init().unwrap_or_else(|error| {
            panic!("初始化 RequestIdGenerator 失败: {error}");
        });
        cell()
            .read()
            .unwrap()
            .as_ref()
            .expect("init 成功后必须存在 RequestIdGenerator")
            .clone()
    }

    /// 生成唯一 Request ID。
    ///
    /// # 返回
    /// 当前配置生成器创建的 Request ID。
    ///
    /// 对应 Java: `IdGeneratorHolder#generate`。
    pub fn generate() -> String {
        Self::load_generator().generate()
    }

    /// 对应 setRequestIdGenerator()：注册自定义生成器
    pub fn register(generator: Arc<dyn RequestIdGenerator>) {
        Self::set_request_id_generator(generator);
    }

    /// 按 Java 配置类名注册 Rust 生成器。
    ///
    /// Rust 没有 JVM 反射，Vernal 装配层可用该入口把容器对象绑定到
    /// `request_id_generator_class` 配置值。
    pub fn register_named(
        request_id_generator_class: impl Into<String>,
        request_id_generator: Arc<dyn RequestIdGenerator>,
    ) {
        named_generators()
            .write()
            .unwrap()
            .insert(request_id_generator_class.into(), request_id_generator);
    }

    /// 清空当前生成器，下次生成 ID 时重新读取 LiteFlow 配置。
    ///
    /// 对应 Java 测试及容器重载时重新执行 `IdGeneratorHolder#init` 的效果。
    pub fn clean() {
        *cell().write().unwrap() = None;
    }
}
