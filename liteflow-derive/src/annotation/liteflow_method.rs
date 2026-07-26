use proc_macro::TokenStream;

/// 声明式组件方法标记。
///
/// 该属性通常由外层 `liteflow_cmp_define` 消费并移除；独立使用时保持方法
/// 原样，避免引入运行期反射。
/// 对应 Java: `com.yomahub.liteflow.annotation.LiteflowMethod`。
pub(crate) fn expand(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}
