//! 流程初始化钩子注册表。
//!
//! 对应 Java: `com.yomahub.liteflow.core.FlowInitHook`。

use std::sync::{Arc, OnceLock, RwLock};

type FlowInitSupplier = Arc<dyn Fn() -> bool + Send + Sync>;

fn suppliers() -> &'static RwLock<Vec<FlowInitSupplier>> {
    static SUPPLIER_LIST: OnceLock<RwLock<Vec<FlowInitSupplier>>> = OnceLock::new();
    SUPPLIER_LIST.get_or_init(|| RwLock::new(Vec::new()))
}

/// 保存第三方中间件规则监听器等无参数初始化动作。
pub struct FlowInitHook;

impl FlowInitHook {
    /// 依次执行当前已注册的全部钩子。
    ///
    /// 对应 Java `FlowInitHook#executeHook`。执行前复制 Arc 快照，避免钩子内部
    /// 再次注册或清理时持有全局写锁。
    pub fn execute_hook() {
        let snapshot = suppliers().read().unwrap().clone();
        for supplier in snapshot {
            let _ = supplier();
        }
    }

    /// 注册一个无参数布尔供应器。返回值与 Java 一样只用于触发，不参与汇总。
    pub fn add_hook<F>(hook_supplier: F)
    where
        F: Fn() -> bool + Send + Sync + 'static,
    {
        suppliers().write().unwrap().push(Arc::new(hook_supplier));
    }

    /// 清空全部钩子。对应 Java `cleanHook()`。
    pub fn clean_hook() {
        suppliers().write().unwrap().clear();
    }

    /// 返回当前钩子数，供运行期诊断和测试使用。
    pub fn len() -> usize {
        suppliers().read().unwrap().len()
    }

    /// 判断当前是否没有初始化钩子。
    pub fn is_empty() -> bool {
        Self::len() == 0
    }
}
