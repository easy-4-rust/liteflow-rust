use crate::spi::context_aware::ContextAware;

/// 本地上下文感知
///
/// 用于获取本地上下文感知实例
///
/// 本类是本地上下文感知的具体实现
///
/// 本类是本地上下文感知的默认实现
#[derive(Clone)]
pub struct LocalContextAware;

impl ContextAware for LocalContextAware {
    /// 获取上下文感知实例
    ///
    /// 返回本地上下文感知实例
    fn get_bean<T: 'static + Send + Sync>(
        &self,
        _context_cmp_init: Option<&dyn crate::spi::context_cmp_init::ContextCmpInit>,
    ) -> T {
        unimplemented!("bean not found")
    }

    /// 注册上下文感知实例
    ///
    /// 将上下文感知实例注册到本地上下文感知实例中
    ///
    /// 如果注册成功，则返回上下文感知实例
    ///
    /// 否则，返回 None
    fn register_bean<T: 'static + Send + Sync>(
        &self,
        _bean_name: &str,
        _bean: T,
    ) -> Option<Arc<dyn std::any::Any + Send + Sync>> {
        unimplemented!("bean not found")
    }
}
