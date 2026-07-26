use proc_macro::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{ItemStruct, LitStr, Result, Token, parse_macro_input};

/// 降级组件声明参数。
///
/// Java 的 `@FallbackCmp` 只负责标记，节点 id 由容器 bean name 提供、节点类型
/// 由父类推断；Rust 无运行期反射，所以在宏参数中显式声明两项元数据。
struct FallbackArgs {
    node_id: LitStr,
    node_type: Option<LitStr>,
}

impl Parse for FallbackArgs {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let node_id = input.parse()?;
        let mut node_type = None;
        while !input.is_empty() {
            input.parse::<Token![,]>()?;
            let key: syn::Ident = input.parse()?;
            input.parse::<Token![=]>()?;
            let value: LitStr = input.parse()?;
            if key == "node_type" {
                node_type = Some(value);
            } else {
                return Err(syn::Error::new(key.span(), "仅支持 node_type"));
            }
        }
        Ok(Self { node_id, node_type })
    }
}

/// 降级组件标记与注册宏。
///
/// 用法：`#[fallback_cmp("commonFallback", node_type = "common")]`。
/// 对应 Java: `com.yomahub.liteflow.annotation.FallbackCmp` 与
/// `FlowBus#addFallbackNode`。
pub(crate) fn expand(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr as FallbackArgs);
    let input = parse_macro_input!(item as ItemStruct);
    if args.node_id.value().trim().is_empty() {
        return syn::Error::new(args.node_id.span(), "fallback component id 不能为空")
            .to_compile_error()
            .into();
    }

    let ident = &input.ident;
    let node_id = args.node_id;
    let node_type = args
        .node_type
        .unwrap_or_else(|| LitStr::new("common", ident.span()));
    let node_type_expr = match node_type.value().as_str() {
        "common" => quote!(::liteflow_core::NodeTypeEnum::Common),
        "switch" => quote!(::liteflow_core::NodeTypeEnum::Switch),
        "boolean" => quote!(::liteflow_core::NodeTypeEnum::Boolean),
        "for" => quote!(::liteflow_core::NodeTypeEnum::For),
        "iterator" => quote!(::liteflow_core::NodeTypeEnum::Iterator),
        _ => {
            return syn::Error::new(
                node_type.span(),
                "fallback node_type 仅支持 common/switch/boolean/for/iterator",
            )
            .to_compile_error()
            .into();
        }
    };

    quote! {
        #input

        impl #ident {
            /// 降级组件节点 id。
            pub const LITEFLOW_FALLBACK_ID: &'static str = #node_id;
            /// 降级组件类型 code。对应 Java FallbackCmp + NodeTypeEnum。
            pub const LITEFLOW_FALLBACK_TYPE: &'static str = #node_type;

            /// 同时注册普通节点 id 与类型降级槽位。
            ///
            /// 对应 Java: `FlowBus#addNode` 后调用 `#addFallbackNode`。
            pub fn register_fallback(
                self,
                bus: &::liteflow_core::FlowBus,
            ) -> ::liteflow_core::LFResult<()> {
                bus.register_fallback(Self::LITEFLOW_FALLBACK_ID, #node_type_expr, self)
            }
        }
    }
    .into()
}
