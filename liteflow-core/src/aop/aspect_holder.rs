//! 对应 Java CmpAroundAspectHolder 的执行期切面快照。

use std::sync::Arc;

use super::ICmpAroundAspect;

/// 一次执行使用的切面注册表。
#[derive(Clone, Default)]
pub struct AspectHolder {
    aspects: Arc<Vec<Arc<dyn ICmpAroundAspect>>>,
}

impl AspectHolder {
    /// 创建空注册表。
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// 注册切面。
    pub fn register(&mut self, aspect: Arc<dyn ICmpAroundAspect>) {
        Arc::get_mut(&mut self.aspects)
            .expect("aspect holder already shared")
            .push(aspect);
    }

    /// 返回已注册切面。
    #[must_use]
    pub fn aspects(&self) -> &[Arc<dyn ICmpAroundAspect>] {
        &self.aspects
    }

    /// 判断是否没有切面。
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.aspects.is_empty()
    }
}
