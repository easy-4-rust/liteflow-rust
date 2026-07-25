//! 对应 Java 类：com.yomahub.liteflow.flow.id.IdGeneratorHolder
//!
//! Id 生成器帮助器。Java 版 init() 读取 LiteflowConfig.requestIdGeneratorClass：
//! 为空则用 DefaultRequestIdGenerator，否则反射实例化并经 ContextAware 注册。
//! Rust 侧与 spi holder 同模式：`OnceLock<RwLock<Option<Arc<dyn RequestIdGenerator>>>>`
//! 全局单例，未注册时回退默认生成器；register() 显式覆盖（对应配置自定义
//! 生成器类的场景——LiteflowConfig 配置读取挂接将在 property 包落地后接入）。

use std::sync::{Arc, OnceLock, RwLock};

use super::default_request_id_generator::DefaultRequestIdGenerator;
use super::request_id_generator::RequestIdGenerator;

static HOLDER: OnceLock<RwLock<Option<Arc<dyn RequestIdGenerator>>>> = OnceLock::new();

fn cell() -> &'static RwLock<Option<Arc<dyn RequestIdGenerator>>> {
    HOLDER.get_or_init(|| RwLock::new(None))
}

/// 对应 IdGeneratorHolder
pub struct IdGeneratorHolder;

impl IdGeneratorHolder {
    /// 对应 getInstance().getRequestIdGenerator()：
    /// 未注册时回退 DefaultRequestIdGenerator（对应 Java init() 中
    /// requestIdGeneratorClass 为空的分支）
    pub fn load_generator() -> Arc<dyn RequestIdGenerator> {
        if let Some(x) = cell().read().unwrap().as_ref() {
            return x.clone();
        }
        let mut guard = cell().write().unwrap();
        guard
            .get_or_insert_with(|| Arc::new(DefaultRequestIdGenerator::new()))
            .clone()
    }

    /// 对应 getInstance().generate()：生成唯一 requestId
    pub fn generate() -> String {
        Self::load_generator().generate()
    }

    /// 对应 setRequestIdGenerator()：注册自定义生成器
    pub fn register(generator: Arc<dyn RequestIdGenerator>) {
        *cell().write().unwrap() = Some(generator);
    }

    /// 清空缓存，下次 load 回退默认生成器
    pub fn clean() {
        *cell().write().unwrap() = None;
    }
}
