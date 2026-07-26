use proc_macro::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{ItemStruct, LitStr, Result, Token, parse_macro_input};

/// LiteFlow 普通组件注册宏。
///
/// 对应 Java: `com.yomahub.liteflow.annotation.LiteflowComponent`。
struct ComponentArgs {
    node_id: LitStr,
    name: Option<LitStr>,
    node_type: Option<LitStr>,
}

impl Parse for ComponentArgs {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let node_id: LitStr = input.parse()?;
        let mut name = None;
        let mut node_type = None;
        while !input.is_empty() {
            input.parse::<Token![,]>()?;
            let key: syn::Ident = input.parse()?;
            input.parse::<Token![=]>()?;
            let value: LitStr = input.parse()?;
            match key.to_string().as_str() {
                "name" => name = Some(value),
                "node_type" => node_type = Some(value),
                _ => return Err(syn::Error::new(key.span(), "仅支持 name 或 node_type")),
            }
        }
        Ok(Self {
            node_id,
            name,
            node_type,
        })
    }
}

pub(crate) fn expand(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr as ComponentArgs);
    let input = parse_macro_input!(item as ItemStruct);
    if args.node_id.value().trim().is_empty() {
        return syn::Error::new(args.node_id.span(), "liteflow component id 不能为空")
            .to_compile_error()
            .into();
    }
    let ident = &input.ident;
    let node_id = args.node_id;
    let name = args.name.unwrap_or_else(|| LitStr::new("", ident.span()));
    let node_type = args
        .node_type
        .unwrap_or_else(|| LitStr::new("common", ident.span()));
    let node_type_expr = match node_type.value().as_str() {
        "common" => quote!(::liteflow_core::NodeTypeEnum::Common),
        "switch" => quote!(::liteflow_core::NodeTypeEnum::Switch),
        "boolean" => quote!(::liteflow_core::NodeTypeEnum::Boolean),
        "for" => quote!(::liteflow_core::NodeTypeEnum::For),
        "iterator" => quote!(::liteflow_core::NodeTypeEnum::Iterator),
        "fallback" => quote!(::liteflow_core::NodeTypeEnum::Fallback),
        _ => {
            return syn::Error::new(node_type.span(), "不支持的 node_type")
                .to_compile_error()
                .into();
        }
    };
    quote! {
        #input

        impl #ident {
            /// 注解声明的节点 id。
            pub const LITEFLOW_NODE_ID: &'static str = #node_id;
            /// 注解声明的节点名称。
            pub const LITEFLOW_NODE_NAME: &'static str = #name;

            /// 把组件注册到 FlowBus。对应 Java 容器扫描后的组件初始化。
            pub fn register(self, bus: &::liteflow_core::FlowBus) -> ::liteflow_core::LFResult<()> {
                ::liteflow_core::LiteFlowNodeBuilder::create_node(bus)
                    .set_id(Self::LITEFLOW_NODE_ID)
                    .set_name(Self::LITEFLOW_NODE_NAME)
                    .set_type(#node_type_expr)
                    .set_component(self)
                    .build()
            }
        }
    }
    .into()
}
