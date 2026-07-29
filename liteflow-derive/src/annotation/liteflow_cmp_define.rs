use std::collections::{BTreeMap, HashSet};

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::ext::IdentExt;
use syn::parse::{Parse, ParseStream};
use syn::{
    ExprArray, FnArg, GenericArgument, ImplItem, ItemImpl, LitInt, LitStr, Pat, PathArguments,
    ReceiverKind, Result, Token, Type, parse_macro_input,
};

/// 声明式组件类级元数据。
///
/// `node_type` 对应 Java `@LiteflowCmpDefine#value`；Rust 还需要显式组件 ID 和名称，
/// 以替代 Spring/Solon bean name 与容器元数据。
struct CmpDefineArgs {
    component_id: LitStr,
    node_name: Option<LitStr>,
    node_type: Option<LitStr>,
}

/// 方法级声明元数据。
///
/// 支持 Java 字段对等写法
/// `#[liteflow_method(value = "process", node_id = "a", ...)]`，并兼容历史
/// `#[liteflow_method("rustDispatchName")]` 写法。
struct LiteflowMethodArgs {
    legacy_method_name: Option<LitStr>,
    value: Option<LitStr>,
    node_id: Option<LitStr>,
    node_name: Option<LitStr>,
    node_type: Option<LitStr>,
}

impl Parse for LiteflowMethodArgs {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        if input.peek(LitStr) {
            return Ok(Self {
                legacy_method_name: Some(input.parse()?),
                value: None,
                node_id: None,
                node_name: None,
                node_type: None,
            });
        }

        let mut args = Self {
            legacy_method_name: None,
            value: None,
            node_id: None,
            node_name: None,
            node_type: None,
        };
        while !input.is_empty() {
            let key: syn::Ident = input.parse()?;
            input.parse::<Token![=]>()?;
            let value: LitStr = input.parse()?;
            let target = match key.to_string().as_str() {
                "value" => &mut args.value,
                "node_id" => &mut args.node_id,
                "node_name" => &mut args.node_name,
                "node_type" => &mut args.node_type,
                _ => {
                    return Err(syn::Error::new(
                        key.span(),
                        "liteflow_method 仅支持 value/node_id/node_name/node_type",
                    ));
                }
            };
            if target.is_some() {
                return Err(syn::Error::new(key.span(), "liteflow_method 字段不能重复"));
            }
            *target = Some(value);
            if !input.is_empty() {
                input.parse::<Token![,]>()?;
            }
        }
        if args.value.is_none() {
            return Err(input.error("liteflow_method 必须声明 value"));
        }
        Ok(args)
    }
}

struct DeclGroup {
    node_id: LitStr,
    node_name: LitStr,
    node_type: LitStr,
    node_type_expr: TokenStream2,
    method_metadata: Vec<TokenStream2>,
    has_main_method: bool,
}

/// 声明式方法上的 Java `@LiteflowRetry` 对等参数。
struct MethodRetryArgs {
    count: LitInt,
    retry_for: Vec<LitStr>,
}

impl Parse for MethodRetryArgs {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let count = input.parse()?;
        let mut retry_for = Vec::new();
        if !input.is_empty() {
            input.parse::<Token![,]>()?;
            let key = syn::Ident::parse_any(input)?;
            if key != "for" {
                return Err(syn::Error::new(key.span(), "仅支持 for = [\"Error\"]"));
            }
            input.parse::<Token![=]>()?;
            let values: ExprArray = input.parse()?;
            for value in values.elems {
                match value {
                    syn::Expr::Lit(expression) => match expression.lit {
                        syn::Lit::Str(value) => retry_for.push(value),
                        _ => {
                            return Err(syn::Error::new_spanned(
                                expression,
                                "for 数组只能包含字符串",
                            ));
                        }
                    },
                    other => {
                        return Err(syn::Error::new_spanned(other, "for 数组只能包含字符串"));
                    }
                }
            }
        }
        Ok(Self { count, retry_for })
    }
}

impl Parse for CmpDefineArgs {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let component_id = input.parse()?;
        let mut node_name = None;
        let mut node_type = None;
        while !input.is_empty() {
            input.parse::<Token![,]>()?;
            let key: syn::Ident = input.parse()?;
            input.parse::<Token![=]>()?;
            let value: LitStr = input.parse()?;
            match key.to_string().as_str() {
                "node_name" => node_name = Some(value),
                "node_type" => node_type = Some(value),
                _ => {
                    return Err(syn::Error::new(
                        key.span(),
                        "liteflow_cmp_define 仅支持 node_name 或 node_type",
                    ));
                }
            }
        }
        Ok(Self {
            component_id,
            node_name,
            node_type,
        })
    }
}

/// 声明式组件定义宏。
///
/// Java 的 `@LiteflowCmpDefine` 依赖运行期反射与 ByteBuddy 生成
/// `NodeComponent` 子类；Rust 在编译期读取 `#[liteflow_method("...")]`，
/// 生成 `DeclComponent#call` 的静态分派表以及显式注册入口。
///
/// 对应 Java:
/// `com.yomahub.liteflow.annotation.LiteflowCmpDefine`、
/// `com.yomahub.liteflow.core.proxy.DeclComponentProxy#getProxy`。
pub(crate) fn expand(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr as CmpDefineArgs);
    let mut input = parse_macro_input!(item as ItemImpl);

    if args.component_id.value().trim().is_empty() {
        return syn::Error::new(args.component_id.span(), "声明式组件 id 不能为空")
            .to_compile_error()
            .into();
    }
    if input.trait_.is_some() {
        return syn::Error::new_spanned(
            &input,
            "liteflow_cmp_define 必须标注在类型的 inherent impl 上",
        )
        .to_compile_error()
        .into();
    }
    if !input.generics.params.is_empty() || input.generics.where_clause.is_some() {
        return syn::Error::new_spanned(
            &input.generics,
            "liteflow_cmp_define 暂不支持带泛型的 impl",
        )
        .to_compile_error()
        .into();
    }

    let self_ty = input.self_ty.clone();
    let component_id = args.component_id;
    let node_name = args
        .node_name
        .unwrap_or_else(|| LitStr::new("", component_id.span()));
    let class_node_type = args.node_type;
    let default_node_type = class_node_type
        .clone()
        .unwrap_or_else(|| LitStr::new("common", component_id.span()));
    let _default_node_type_expr = match node_type_expr(&default_node_type) {
        Ok(value) => value,
        Err(error) => return error.to_compile_error().into(),
    };
    let default_main_method_expr = main_method_for_node_type(&default_node_type);
    let mut dispatches = Vec::new();
    let mut error_dispatches = Vec::new();
    let mut method_names = HashSet::new();
    let mut groups: BTreeMap<String, DeclGroup> = BTreeMap::new();

    for impl_item in &mut input.items {
        let ImplItem::Fn(method) = impl_item else {
            continue;
        };
        let mut liteflow_args = None;
        let mut method_retry = None;
        let mut retained_attrs = Vec::with_capacity(method.attrs.len());
        for annotation in std::mem::take(&mut method.attrs) {
            if annotation.path().is_ident("liteflow_method") {
                if liteflow_args.is_some() {
                    return syn::Error::new_spanned(
                        annotation,
                        "同一方法不能重复标注 liteflow_method",
                    )
                    .to_compile_error()
                    .into();
                }
                match annotation.parse_args::<LiteflowMethodArgs>() {
                    Ok(args) => liteflow_args = Some(args),
                    Err(error) => return error.to_compile_error().into(),
                }
            } else if annotation.path().is_ident("liteflow_retry") {
                if method_retry.is_some() {
                    return syn::Error::new_spanned(
                        annotation,
                        "同一方法不能重复标注 liteflow_retry",
                    )
                    .to_compile_error()
                    .into();
                }
                match annotation.parse_args::<MethodRetryArgs>() {
                    Ok(args) => method_retry = Some(args),
                    Err(error) => return error.to_compile_error().into(),
                }
            } else {
                retained_attrs.push(annotation);
            }
        }
        method.attrs = retained_attrs;

        let Some(method_args) = liteflow_args else {
            continue;
        };
        let rust_method = &method.sig.ident;
        let method_name = method_args
            .legacy_method_name
            .clone()
            .unwrap_or_else(|| LitStr::new(&rust_method.to_string(), rust_method.span()));
        if method_name.value().trim().is_empty() {
            return syn::Error::new(method_name.span(), "liteflow_method 名称不能为空")
                .to_compile_error()
                .into();
        }
        if !method_names.insert(method_name.value()) {
            return syn::Error::new(
                method_name.span(),
                format!("重复的 liteflow_method 名称: {}", method_name.value()),
            )
            .to_compile_error()
            .into();
        }

        let (liteflow_method_expr, is_main_method) = if method_args.legacy_method_name.is_some() {
            (default_main_method_expr.clone(), true)
        } else {
            match liteflow_method_expr(method_args.value.as_ref().expect("value checked")) {
                Ok(value) => value,
                Err(error) => return error.to_compile_error().into(),
            }
        };
        let is_on_error = method_args
            .value
            .as_ref()
            .is_some_and(|value| normalize_liteflow_method(value) == "onerror");
        let method_node_type = class_node_type
            .clone()
            .or(method_args.node_type)
            .unwrap_or_else(|| LitStr::new("common", method_name.span()));
        let method_node_type_expr = match node_type_expr(&method_node_type) {
            Ok(value) => value,
            Err(error) => return error.to_compile_error().into(),
        };
        let group_node_id = method_args.node_id.unwrap_or_else(|| component_id.clone());
        if group_node_id.value().trim().is_empty() {
            return syn::Error::new(group_node_id.span(), "liteflow_method node_id 不能为空")
                .to_compile_error()
                .into();
        }
        let group_node_name = method_args.node_name.unwrap_or_else(|| node_name.clone());
        if method.sig.asyncness.is_none() {
            return syn::Error::new_spanned(
                &method.sig,
                "liteflow_method 对应的方法必须声明为 async fn",
            )
            .to_compile_error()
            .into();
        }

        let mut args = method.sig.inputs.iter_mut();
        if !matches!(
            args.next(),
            Some(FnArg::Receiver(receiver))
                if matches!(&receiver.kind, ReceiverKind::Reference(..))
        ) {
            return syn::Error::new_spanned(
                &method.sig,
                "liteflow_method 的第一个参数必须是 &self",
            )
            .to_compile_error()
            .into();
        }
        if !matches!(args.next(), Some(FnArg::Typed(_))) {
            return syn::Error::new_spanned(
                &method.sig,
                "liteflow_method 必须在 &self 后接收 CmpContext 参数",
            )
            .to_compile_error()
            .into();
        }

        // Java DeclComponentProxy#loadMethodParameter 会按 @LiteflowFact 从 Slot
        // contextBeanList 中取事实对象。Rust 端要求 Arc<T>，在编译期生成强类型注入。
        let mut fact_bindings = Vec::new();
        let mut invocation_arguments: Vec<TokenStream2> = Vec::new();
        let mut parameter_metadata = Vec::new();
        let mut has_error_parameter = false;
        for argument in args {
            let FnArg::Typed(argument) = argument else {
                return syn::Error::new_spanned(
                    argument,
                    "liteflow_fact 参数必须位于 CmpContext 参数之后",
                )
                .to_compile_error()
                .into();
            };
            let mut fact_name = None;
            let mut retained_attrs = Vec::with_capacity(argument.attrs.len());
            for annotation in std::mem::take(&mut argument.attrs) {
                if annotation.path().is_ident("liteflow_fact") {
                    if fact_name.is_some() {
                        return syn::Error::new_spanned(
                            annotation,
                            "同一参数不能重复标注 liteflow_fact",
                        )
                        .to_compile_error()
                        .into();
                    }
                    match annotation.parse_args::<LitStr>() {
                        Ok(name) => fact_name = Some(name),
                        Err(error) => return error.to_compile_error().into(),
                    }
                } else {
                    retained_attrs.push(annotation);
                }
            }
            argument.attrs = retained_attrs;

            let parameter_index = invocation_arguments.len() + 1;
            let parameter_type = argument.ty.as_ref().clone();
            if fact_name.is_none() && is_on_error && !has_error_parameter {
                has_error_parameter = true;
                invocation_arguments.push(quote!(error));
                parameter_metadata.push(quote! {
                    ::liteflow_core::core::proxy::ParameterWrapBean::new(
                        ::std::any::type_name::<#parameter_type>(),
                        None::<&str>,
                        #parameter_index,
                    )
                });
                continue;
            }

            let Some(fact_name) = fact_name else {
                return syn::Error::new_spanned(
                    argument,
                    "CmpContext 之后仅 ON_ERROR 的首个参数可接收 &LiteflowError，其余参数必须标注 #[liteflow_fact(\"beanName\")]",
                )
                .to_compile_error()
                .into();
            };
            if fact_name.value().trim().is_empty() {
                return syn::Error::new(fact_name.span(), "liteflow_fact bean 名称不能为空")
                    .to_compile_error()
                    .into();
            }
            let Pat::Ident(pattern) = argument.pat.as_ref() else {
                return syn::Error::new_spanned(
                    &argument.pat,
                    "liteflow_fact 参数必须使用简单标识符",
                )
                .to_compile_error()
                .into();
            };
            let fact_ident = pattern.ident.clone();
            let fact_type = parameter_type;
            let fact_inner_type = match arc_inner_type(&fact_type) {
                Ok(inner) => inner,
                Err(error) => return error.to_compile_error().into(),
            };
            fact_bindings.push(quote! {
                let #fact_ident: #fact_type = ctx
                    .bean::<#fact_inner_type>(#fact_name)
                    .ok_or_else(|| {
                        ::liteflow_core::LiteflowError::CmpDefine(format!(
                            "decl component[{}] method[{}] fact bean[{}] not found",
                            Self::LITEFLOW_DECL_ID,
                            #method_name,
                            #fact_name,
                        ))
                    })?;
            });
            parameter_metadata.push(quote! {
                ::liteflow_core::core::proxy::ParameterWrapBean::new(
                    ::std::any::type_name::<#fact_type>(),
                    Some(#fact_name),
                    #parameter_index,
                )
            });
            invocation_arguments.push(quote!(#fact_ident));
        }

        if is_on_error {
            error_dispatches.push(quote! {
                #method_name => {
                    #(#fact_bindings)*
                    self.#rust_method(ctx, #(#invocation_arguments),*).await
                },
            });
            dispatches.push(quote! {
                #method_name => Err(::liteflow_core::LiteflowError::CmpDefine(
                    format!(
                        "decl component[{}] method[{}] requires error context",
                        Self::LITEFLOW_DECL_ID,
                        #method_name,
                    ),
                )),
            });
        } else {
            dispatches.push(quote! {
                #method_name => {
                    #(#fact_bindings)*
                    self.#rust_method(ctx, #(#invocation_arguments),*).await
                },
            });
        }
        let (retry_count, retry_for) = method_retry.map_or_else(
            || (quote!(None), Vec::new()),
            |retry| {
                let count = retry.count;
                (quote!(Some(#count)), retry.retry_for)
            },
        );
        let metadata = quote! {
            ::liteflow_core::core::proxy::MethodWrapBean::new(
                ::liteflow_core::core::proxy::LiteFlowMethodBean::new(
                    #method_name,
                    #liteflow_method_expr,
                ),
                #liteflow_method_expr,
                #method_node_type_expr,
                #retry_count,
                ::std::vec![#(::std::string::String::from(#retry_for)),*],
                ::std::vec![#(#parameter_metadata),*],
            )
        };

        let group_key = group_node_id.value();
        let group = groups
            .entry(group_key.clone())
            .or_insert_with(|| DeclGroup {
                node_id: group_node_id.clone(),
                node_name: group_node_name.clone(),
                node_type: method_node_type.clone(),
                node_type_expr: method_node_type_expr.clone(),
                method_metadata: Vec::new(),
                has_main_method: false,
            });
        if group.node_type.value() != method_node_type.value() {
            return syn::Error::new(
                method_node_type.span(),
                format!(
                    "同一 node_id[{}] 的 liteflow_method 必须声明相同 node_type",
                    group_key
                ),
            )
            .to_compile_error()
            .into();
        }
        if is_main_method {
            if group.has_main_method {
                return syn::Error::new(
                    method_name.span(),
                    format!("node_id[{}] 不能重复声明主方法", group_key),
                )
                .to_compile_error()
                .into();
            }
            // Java 以主方法的 nodeName/nodeType 填充 DeclWarpBean。
            group.node_name = group_node_name;
            group.node_type = method_node_type;
            group.node_type_expr = method_node_type_expr;
            group.has_main_method = true;
        }
        group.method_metadata.push(metadata);
    }

    if groups.is_empty() {
        return syn::Error::new_spanned(
            &input,
            "liteflow_cmp_define 至少需要一个 #[liteflow_method(\"...\")] 方法",
        )
        .to_compile_error()
        .into();
    }

    for group in groups.values() {
        if !group.has_main_method {
            return syn::Error::new(
                group.node_id.span(),
                format!(
                    "Component [{}] does not define the process method",
                    group.node_id.value()
                ),
            )
            .to_compile_error()
            .into();
        }
    }

    let group_registrations = groups.values().map(|group| {
        let node_id = &group.node_id;
        let node_name = &group.node_name;
        let node_type_expr = &group.node_type_expr;
        let method_metadata = &group.method_metadata;
        quote! {
            let decl_warp_bean =
                ::liteflow_core::core::proxy::DeclWarpBean::new(
                    #node_id,
                    #node_name,
                    #node_type_expr,
                    raw_bean.clone(),
                    ::std::any::type_name::<Self>(),
                    ::std::vec![#(#method_metadata),*],
                );
            bus.register_decl_warp(decl_warp_bean);
        }
    });
    let group_ids = groups
        .values()
        .map(|group| &group.node_id)
        .collect::<Vec<_>>();
    let group_count = group_ids.len();
    let group_id_const = format_ident!("LITEFLOW_DECL_NODE_IDS");

    quote! {
        #input

        #[::liteflow_core::async_trait]
        impl ::liteflow_core::core::decl_component::DeclComponent for #self_ty {
            /// 按 LiteFlow 方法名分派到声明式组件方法。
            ///
            /// 对应 Java: `DeclComponentProxy.AopInvocationHandler#invoke`。
            async fn call(
                &self,
                method: &str,
                ctx: &::liteflow_core::CmpContext,
            ) -> Result<
                ::liteflow_core::serde_json::Value,
                ::liteflow_core::LiteflowError,
            > {
                match method {
                    #(#dispatches)*
                    other => Err(::liteflow_core::LiteflowError::CmpDefine(
                        format!(
                            "decl component[{}] has no liteflow method[{other}]",
                            Self::LITEFLOW_DECL_ID,
                        ),
                    )),
                }
            }

            async fn call_with_error(
                &self,
                method: &str,
                ctx: &::liteflow_core::CmpContext,
                error: &::liteflow_core::LiteflowError,
            ) -> Result<
                ::liteflow_core::serde_json::Value,
                ::liteflow_core::LiteflowError,
            > {
                match method {
                    #(#error_dispatches)*
                    other => self.call(other, ctx).await,
                }
            }
        }

        impl #self_ty {
            /// 声明式组件 id。
            pub const LITEFLOW_DECL_ID: &'static str = #component_id;
            /// 声明式组件名称。
            pub const LITEFLOW_DECL_NAME: &'static str = #node_name;
            /// 声明式组件类型代码，对应 Java `LiteflowCmpDefine#value`。
            pub const LITEFLOW_DECL_NODE_TYPE: &'static str = #default_node_type;
            /// 当前声明对象按 Java `nodeId` 分组后生成的全部节点 ID。
            pub const #group_id_const: [&'static str; #group_count] = [#(#group_ids),*];

            /// 生成包装元数据、创建代理并注册声明式组件。
            ///
            /// 对应 Java `DeclComponentParser#parseDeclBean`、
            /// `LiteFlowProxyUtil#proxy2NodeComponent`。
            pub fn register_decl(self, bus: &::liteflow_core::FlowBus) {
                let raw_bean: ::std::sync::Arc<
                    dyn ::liteflow_core::core::decl_component::DeclComponent
                > = ::std::sync::Arc::new(self);
                #(#group_registrations)*
            }
        }
    }
    .into()
}

/// 将 Rust 注解中的节点类型代码转换为核心枚举表达式。
fn node_type_expr(node_type: &LitStr) -> syn::Result<TokenStream2> {
    match node_type.value().to_ascii_lowercase().as_str() {
        "common" => Ok(quote!(::liteflow_core::enums::NodeTypeEnum::Common)),
        "switch" => Ok(quote!(::liteflow_core::enums::NodeTypeEnum::Switch)),
        "boolean" => Ok(quote!(::liteflow_core::enums::NodeTypeEnum::Boolean)),
        "for" => Ok(quote!(::liteflow_core::enums::NodeTypeEnum::For)),
        "iterator" => Ok(quote!(::liteflow_core::enums::NodeTypeEnum::Iterator)),
        _ => Err(syn::Error::new(
            node_type.span(),
            "声明式组件 node_type 仅支持 common/switch/boolean/for/iterator",
        )),
    }
}

/// 按节点类型返回历史简写对应的主方法角色。
fn main_method_for_node_type(node_type: &LitStr) -> TokenStream2 {
    match node_type.value().to_ascii_lowercase().as_str() {
        "switch" => quote!(::liteflow_core::enums::LiteFlowMethodEnum::ProcessSwitch),
        "boolean" => quote!(::liteflow_core::enums::LiteFlowMethodEnum::ProcessBoolean),
        "for" => quote!(::liteflow_core::enums::LiteFlowMethodEnum::ProcessFor),
        "iterator" => quote!(::liteflow_core::enums::LiteFlowMethodEnum::ProcessIterator),
        _ => quote!(::liteflow_core::enums::LiteFlowMethodEnum::Process),
    }
}

/// 将 Java `LiteFlowMethodEnum` 名称或 Java 方法名转换为核心枚举表达式。
fn liteflow_method_expr(value: &LitStr) -> syn::Result<(TokenStream2, bool)> {
    let normalized = normalize_liteflow_method(value);
    let (method, is_main) = match normalized.as_str() {
        "process" => (
            quote!(::liteflow_core::enums::LiteFlowMethodEnum::Process),
            true,
        ),
        "processswitch" => (
            quote!(::liteflow_core::enums::LiteFlowMethodEnum::ProcessSwitch),
            true,
        ),
        "processboolean" => (
            quote!(::liteflow_core::enums::LiteFlowMethodEnum::ProcessBoolean),
            true,
        ),
        "processfor" => (
            quote!(::liteflow_core::enums::LiteFlowMethodEnum::ProcessFor),
            true,
        ),
        "processiterator" => (
            quote!(::liteflow_core::enums::LiteFlowMethodEnum::ProcessIterator),
            true,
        ),
        "isaccess" => (
            quote!(::liteflow_core::enums::LiteFlowMethodEnum::IsAccess),
            false,
        ),
        "isend" => (
            quote!(::liteflow_core::enums::LiteFlowMethodEnum::IsEnd),
            false,
        ),
        "iscontinueonerror" => (
            quote!(::liteflow_core::enums::LiteFlowMethodEnum::IsContinueOnError),
            false,
        ),
        "getnodeexecutorclass" => (
            quote!(::liteflow_core::enums::LiteFlowMethodEnum::GetNodeExecutorClass),
            false,
        ),
        "onsuccess" => (
            quote!(::liteflow_core::enums::LiteFlowMethodEnum::OnSuccess),
            false,
        ),
        "onerror" => (
            quote!(::liteflow_core::enums::LiteFlowMethodEnum::OnError),
            false,
        ),
        "beforeprocess" => (
            quote!(::liteflow_core::enums::LiteFlowMethodEnum::BeforeProcess),
            false,
        ),
        "afterprocess" => (
            quote!(::liteflow_core::enums::LiteFlowMethodEnum::AfterProcess),
            false,
        ),
        "getdisplayname" => (
            quote!(::liteflow_core::enums::LiteFlowMethodEnum::GetDisplayName),
            false,
        ),
        "rollback" => (
            quote!(::liteflow_core::enums::LiteFlowMethodEnum::Rollback),
            false,
        ),
        _ => {
            return Err(syn::Error::new(
                value.span(),
                "未知 LiteFlowMethodEnum；请使用 process/process_switch/is_access 等 Java 对等角色",
            ));
        }
    };
    Ok((method, is_main))
}

/// 归一化 Java 枚举名、camelCase 方法名和 Rust snake_case 写法。
fn normalize_liteflow_method(value: &LitStr) -> String {
    value
        .value()
        .chars()
        .filter(|character| *character != '_')
        .flat_map(char::to_lowercase)
        .collect()
}

/// 从 `Arc<T>` 参数中提取 T。Java 运行期直接传 bean 引用；Rust 为保证跨异步
/// 调用的所有权安全，声明式事实参数统一使用 `Arc<T>`。
fn arc_inner_type(ty: &Type) -> syn::Result<Type> {
    let Type::Path(type_path) = ty else {
        return Err(syn::Error::new_spanned(
            ty,
            "liteflow_fact 参数类型必须是 Arc<T>",
        ));
    };
    let Some(segment) = type_path.path.segments.last() else {
        return Err(syn::Error::new_spanned(
            ty,
            "liteflow_fact 参数类型必须是 Arc<T>",
        ));
    };
    if segment.ident != "Arc" {
        return Err(syn::Error::new_spanned(
            ty,
            "liteflow_fact 参数类型必须是 Arc<T>",
        ));
    }
    let PathArguments::AngleBracketed(arguments) = &segment.arguments else {
        return Err(syn::Error::new_spanned(
            ty,
            "liteflow_fact 参数类型必须是 Arc<T>",
        ));
    };
    match arguments.args.first() {
        Some(GenericArgument::Type(inner)) if arguments.args.len() == 1 => Ok(inner.clone()),
        _ => Err(syn::Error::new_spanned(
            ty,
            "liteflow_fact 参数类型必须是 Arc<T>",
        )),
    }
}
