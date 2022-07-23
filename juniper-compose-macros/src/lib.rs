#![warn(clippy::all)]
#![warn(clippy::pedantic)]
#![allow(clippy::missing_panics_doc)]

use heck::ToLowerCamelCase;
use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{
    parenthesized,
    parse::Parse,
    parse2, parse_macro_input,
    punctuated::Punctuated,
    token::{Comma, Paren},
    Error, Ident, ImplItem, ItemImpl, LitStr, Path, Result, Token, Type,
};

#[proc_macro_attribute]
pub fn composable_object(
    _: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let item_impl = parse_macro_input!(item as ItemImpl);
    expand_composable_object(&item_impl).into()
}

#[proc_macro]
pub fn composite_object(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as CompositeObjectInput);
    let context = input
        .context_ty
        .map_or_else(|| parse2(quote! { () }).unwrap(), |input| input.ty);
    expand_composite_object(&input.ident, &context, &input.composables).into()
}

struct CompositeObjectInput {
    ident: Ident,
    context_ty: Option<CompositeObjectCustomContextType>,
    #[allow(dead_code)]
    paren: Paren,
    composables: Punctuated<Path, Comma>,
}

impl Parse for CompositeObjectInput {
    fn parse(input: syn::parse::ParseStream) -> Result<Self> {
        let ident = input.parse()?;
        let context_ty = if input.peek(Token![<]) {
            Some(input.parse()?)
        } else {
            None
        };
        let composables;
        let paren = parenthesized!(composables in input);
        Ok(Self {
            ident,
            context_ty,
            paren,
            composables: composables.parse_terminated(Path::parse)?,
        })
    }
}

struct CompositeObjectCustomContextType {
    #[allow(dead_code)]
    left_angle_bracket: Token![<],
    #[allow(dead_code)]
    context_ident: Ident,
    #[allow(dead_code)]
    eq_token: Token![=],
    ty: Type,
    #[allow(dead_code)]
    right_angle_bracket: Token![>],
}

impl Parse for CompositeObjectCustomContextType {
    fn parse(input: syn::parse::ParseStream) -> Result<Self> {
        let left_angle_bracket = input.parse()?;
        let context_ident = input.parse::<Ident>()?;
        if context_ident != "Context" {
            return Err(Error::new(context_ident.span(), "expected `Context`"));
        }
        let eq_token = input.parse()?;
        let ty = input.parse()?;
        let right_angle_bracket = input.parse()?;
        Ok(Self {
            left_angle_bracket,
            context_ident,
            eq_token,
            ty,
            right_angle_bracket,
        })
    }
}

fn expand_composable_object(item_impl: &ItemImpl) -> TokenStream {
    let ty = &item_impl.self_ty;

    let fields = item_impl
        .items
        .iter()
        .filter_map(|item| {
            if let ImplItem::Method(method) = item {
                Some(method)
            } else {
                None
            }
        })
        .map(|method| {
            LitStr::new(
                &method.sig.ident.to_string().to_lower_camel_case(),
                Span::call_site(),
            )
        });

    quote! {
        impl ::juniper_compose::ComposableObject for #ty {
            fn fields() -> &'static [&'static str] {
                &[#( #fields ),*]
            }
        }

        #item_impl
    }
}

fn expand_composite_object<P>(
    name: &Ident,
    context: &Type,
    composables: &Punctuated<Path, P>,
) -> TokenStream {
    let name_lit = LitStr::new(&name.to_string(), Span::call_site());
    let impl_graphql_type = expand_impl_graphql_type(name, &name_lit, composables.iter());
    let impl_graphql_value =
        expand_impl_graphql_value(name, &name_lit, context, composables.iter());
    let impl_graphql_value_async =
        expand_impl_graphql_value_async(name, &name_lit, composables.iter());
    quote! {
        #[derive(::std::default::Default)]
        struct #name;
        #impl_graphql_type
        #impl_graphql_value
        #impl_graphql_value_async
    }
}

fn expand_impl_graphql_type<'a>(
    name: &Ident,
    name_lit: &LitStr,
    composables: impl IntoIterator<Item = &'a Path>,
) -> TokenStream {
    let composables = composables.into_iter();
    quote! {
        impl ::juniper::GraphQLType for #name {
            fn name(info: &Self::TypeInfo) -> ::std::option::Option<&str> {
                ::std::option::Option::Some(#name_lit)
            }

            fn meta<'r>(
                info: &Self::TypeInfo,
                registry: &mut ::juniper::executor::Registry<'r, ::juniper::DefaultScalarValue>
            ) -> ::juniper::meta::MetaType<'r, ::juniper::DefaultScalarValue>
            where
                ::juniper::DefaultScalarValue: 'r
            {
                let mut fields = ::std::vec![];
                let mut seen_field_names = ::std::collections::HashSet::<&str>::new();

                #(
                    let composable_meta = <#composables as ::juniper::GraphQLType>::meta(info, registry);

                    for field_name in <#composables as ::juniper_compose::ComposableObject>::fields() {
                        if !seen_field_names.insert(field_name) {
                            ::std::panic!("Conflicting field in composed objects: {}", field_name);
                        }

                        let composable_field = composable_meta
                            .field_by_name(field_name)
                            .unwrap_or_else(|| {
                                ::std::panic!(
                                    "Incorrect implementation of ComposableObject on type {}: unknown field {}",
                                    <#composables as ::juniper::GraphQLType>::name(&()).unwrap_or("<anonymous>"), field_name
                                )
                            });

                        fields.push(::juniper::meta::Field {
                            name: composable_field.name.clone(),
                            description: composable_field.description.clone(),
                            arguments: composable_field.arguments.as_ref().map(|arguments| {
                                arguments
                                    .iter()
                                    .map(|argument| ::juniper::meta::Argument {
                                        name: argument.name.clone(),
                                        description: argument.description.clone(),
                                        arg_type: ::juniper_compose::type_to_owned(&argument.arg_type),
                                        default_value: argument.default_value.clone(),
                                    })
                                    .collect()
                            }),
                            field_type: ::juniper_compose::type_to_owned(&composable_field.field_type),
                            deprecation_status: composable_field.deprecation_status.clone(),
                        });
                    }
                )*

                registry.build_object_type::<Self>(&(), &fields).into_meta()
            }
        }
    }
}

fn expand_impl_graphql_value<'a>(
    name: &Ident,
    name_lit: &LitStr,
    context: &Type,
    composables: impl IntoIterator<Item = &'a Path>,
) -> TokenStream {
    let composables = composables.into_iter();
    quote! {
        impl ::juniper::GraphQLValue for #name {
            type Context = #context;
            type TypeInfo = ();

            fn type_name<'i>(&self, info: &'i Self::TypeInfo) -> Option<&'i str> {
                <Self as ::juniper::GraphQLType>::name(info)
            }

            fn resolve_field(
                &self,
                info: &Self::TypeInfo,
                field_name: &str,
                arguments: &::juniper::Arguments<'_, ::juniper::DefaultScalarValue>,
                executor: &::juniper::executor::Executor<'_, '_, Self::Context, ::juniper::DefaultScalarValue>
            ) -> ::juniper::executor::ExecutionResult<::juniper::DefaultScalarValue> {
                #(
                    if <#composables as ::juniper_compose::ComposableObject>::fields().contains(&field_name) {
                        return <#composables as ::juniper::GraphQLValue>::resolve_field(
                            &<#composables as ::std::default::Default>::default(),
                            info,
                            field_name,
                            arguments,
                            executor
                        );
                    }
                )*
                Err(::juniper::FieldError::from(::std::format!(
                    "Field `{}` not found on type `{}`",
                    field_name,
                    #name_lit,
                )))
            }

            fn concrete_type_name(
                &self,
                context: &Self::Context,
                info: &Self::TypeInfo
            ) -> String {
                String::from(#name_lit)
            }
        }
    }
}

fn expand_impl_graphql_value_async<'a>(
    name: &Ident,
    name_lit: &LitStr,
    composables: impl IntoIterator<Item = &'a Path>,
) -> TokenStream {
    let composables = composables.into_iter();
    quote! {
        impl ::juniper::GraphQLValueAsync for #name
        where
            Self::TypeInfo: Sync,
            Self::Context: Sync,
        {
            fn resolve_field_async<'a>(
                &'a self,
                info: &'a Self::TypeInfo,
                field_name: &'a str,
                arguments: &'a ::juniper::Arguments<'_, ::juniper::DefaultScalarValue>,
                executor: &'a ::juniper::executor::Executor<'_, '_, Self::Context, ::juniper::DefaultScalarValue>
            ) -> ::juniper::BoxFuture<'a, ::juniper::executor::ExecutionResult<::juniper::DefaultScalarValue>> {
                #(
                    if <#composables as ::juniper_compose::ComposableObject>::fields().contains(&field_name) {
                        return ::std::boxed::Box::pin(async move {
                            <#composables as ::juniper::GraphQLValueAsync>::resolve_field_async(
                                &<#composables as ::std::default::Default>::default(),
                                info,
                                field_name,
                                arguments,
                                executor
                            ).await
                        })
                    }
                )*
                ::std::boxed::Box::pin(async move { Err(::juniper::FieldError::from(::std::format!(
                    "Field `{}` not found on type `{}`",
                    field_name,
                    #name_lit,
                ))) })
            }
        }
    }
}
