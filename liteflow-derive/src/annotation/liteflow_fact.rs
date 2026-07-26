use proc_macro::TokenStream;

/// 声明式组件事实参数标记。
///
/// 该属性由外层 `liteflow_cmp_define` 在编译期消费；它把指定名称的
/// `CmpContext` bean 注入为 `Arc<T>` 参数。独立使用时保持原 item。
/// 对应 Java: `com.yomahub.liteflow.annotation.LiteflowFact`。
pub(crate) fn expand(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}
