use std::collections::HashSet;

use proc_macro::TokenStream;
use quote::quote;
use syn::{
    FnArg, GenericArgument, ImplItem, ItemImpl, LitStr, Pat, PathArguments, Type, parse_macro_input,
};

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
    let component_id = parse_macro_input!(attr as LitStr);
    let mut input = parse_macro_input!(item as ItemImpl);

    if component_id.value().trim().is_empty() {
        return syn::Error::new(component_id.span(), "声明式组件 id 不能为空")
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
    let mut dispatches = Vec::new();
    let mut method_names = HashSet::new();

    for impl_item in &mut input.items {
        let ImplItem::Fn(method) = impl_item else {
            continue;
        };
        let mut liteflow_name = None;
        let mut retained_attrs = Vec::with_capacity(method.attrs.len());
        for annotation in std::mem::take(&mut method.attrs) {
            if annotation.path().is_ident("liteflow_method") {
                if liteflow_name.is_some() {
                    return syn::Error::new_spanned(
                        annotation,
                        "同一方法不能重复标注 liteflow_method",
                    )
                    .to_compile_error()
                    .into();
                }
                match annotation.parse_args::<LitStr>() {
                    Ok(name) => liteflow_name = Some(name),
                    Err(error) => return error.to_compile_error().into(),
                }
            } else {
                retained_attrs.push(annotation);
            }
        }
        method.attrs = retained_attrs;

        let Some(method_name) = liteflow_name else {
            continue;
        };
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
        if method.sig.asyncness.is_none() {
            return syn::Error::new_spanned(
                &method.sig,
                "liteflow_method 对应的方法必须声明为 async fn",
            )
            .to_compile_error()
            .into();
        }

        let mut args = method.sig.inputs.iter_mut();
        if !matches!(args.next(), Some(FnArg::Receiver(receiver)) if receiver.reference.is_some()) {
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
        let mut fact_arguments = Vec::new();
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

            let Some(fact_name) = fact_name else {
                return syn::Error::new_spanned(
                    argument,
                    "CmpContext 之后的参数必须标注 #[liteflow_fact(\"beanName\")]",
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
            let fact_type = argument.ty.as_ref().clone();
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
            fact_arguments.push(fact_ident);
        }

        let rust_method = &method.sig.ident;
        dispatches.push(quote! {
            #method_name => {
                #(#fact_bindings)*
                self.#rust_method(ctx, #(#fact_arguments),*).await
            },
        });
    }

    if dispatches.is_empty() {
        return syn::Error::new_spanned(
            &input,
            "liteflow_cmp_define 至少需要一个 #[liteflow_method(\"...\")] 方法",
        )
        .to_compile_error()
        .into();
    }

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
        }

        impl #self_ty {
            /// 声明式组件 id。
            pub const LITEFLOW_DECL_ID: &'static str = #component_id;

            /// 注册声明式组件。对应 Java 容器的 `DeclComponentParser#parseDeclBean`。
            pub fn register_decl(self, bus: &::liteflow_core::FlowBus) {
                bus.register_decl(Self::LITEFLOW_DECL_ID, ::std::sync::Arc::new(self));
            }
        }
    }
    .into()
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
