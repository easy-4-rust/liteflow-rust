//! EL 包装器转换协议。

use super::{BoxedELWrapper, CommonNodeELWrapper, ELWrapper};

/// 把 Rust 字符串或具体包装器转换为 Java `ELWrapper` 等价 trait object。
pub trait IntoELWrapper {
    /// 消费输入并返回盒装 EL 包装器。
    fn into_el_wrapper(self) -> BoxedELWrapper;
}

impl<T> IntoELWrapper for T
where
    T: ELWrapper + 'static,
{
    fn into_el_wrapper(self) -> BoxedELWrapper {
        Box::new(self)
    }
}

impl IntoELWrapper for String {
    fn into_el_wrapper(self) -> BoxedELWrapper {
        Box::new(CommonNodeELWrapper::new(self))
    }
}

impl IntoELWrapper for &str {
    fn into_el_wrapper(self) -> BoxedELWrapper {
        Box::new(CommonNodeELWrapper::new(self))
    }
}
