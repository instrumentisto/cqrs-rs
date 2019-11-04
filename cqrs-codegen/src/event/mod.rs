//! Codegen for [`cqrs::Event`] and related (e.g., [`cqrs::VersionedEvent`], etc).

mod event;
mod versioned_event;

pub(crate) use event::derive as derive;
pub(crate) use versioned_event::derive as versioned_derive;

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Error, Result, spanned::Spanned};
use synstructure::Structure;

use crate::util::{self, TryInto as _};

const ATTR_NAME: &str = "event";
const VALID_ATTR_ARGS: &[&str] = &["type", "version"];

/// Renders implementation of trait `trait_path` with given `body` and
/// optionally renders some arbitrary impl block code with given `optional`.
fn render_struct(
    input: &syn::DeriveInput,
    trait_path: TokenStream,
    body: TokenStream,
    optional: Option<TokenStream>,
) -> Result<TokenStream> {
    let type_name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let optional = optional.map(|optional| {
        quote! {
            #[automatically_derived]
            impl#impl_generics #type_name#ty_generics #where_clause {
                #optional
            }
        }
    });

    Ok(quote! {
        #optional

        #[automatically_derived]
        impl#impl_generics #trait_path for #type_name#ty_generics #where_clause {
            #body
        }
    })
}

/// Checks that all variants of `structure` contain exactly one field. Returns error otherwise.
///
/// `trait_name` is only used to generate error message.
fn assert_all_enum_variants_have_single_field(
    structure: &Structure,
    trait_name: &str,
) -> Result<()> {
    for variant in structure.variants() {
        let ast = variant.ast();
        if ast.fields.len() != 1 {
            return Err(Error::new(
                ast.ident.span(),
                format!(
                    "{} can only be derived for enums with variants that have exactly one field",
                    trait_name
                ),
            ));
        }
    }

    Ok(())
}

/// Renders implementation of trait with path `trait_path` as a `method`
/// that proxies call to it's variants.
///
/// Expects that all variants of `structure` contain exactly one field. Returns error otherwise.
///
/// `trait_name` is only used to generate error message.
fn render_enum_proxy_method_calls(
    structure: &mut Structure,
    trait_name: &str,
    trait_path: TokenStream,
    method: TokenStream,
    method_return_type: TokenStream,
) -> Result<TokenStream> {
    assert_all_enum_variants_have_single_field(&structure, trait_name)?;

    structure.add_bounds(synstructure::AddBounds::Fields);

    structure.binding_name(|field, _| {
        field.ident.as_ref().map_or_else(
            || syn::Ident::new("ev", proc_macro2::Span::call_site()),
            |ident| ident.clone(),
        )
    });

    let body = structure.each(|binding_info| {
        let ev = &binding_info.binding;
        quote!(#ev.#method())
    });

    Ok(
        structure.gen_impl(quote! {
            #[automatically_derived]
            gen impl #trait_path for @Self {
                fn #method(&self) -> #method_return_type {
                    match *self {
                        #body
                    }
                }
            }
        })
    )
}

/// Parses required inner attribute from `#[event(...)]` outer attribute,
/// converting it to type `T` (using [`crate::util::TryInto`]) if possible.
fn parse_attr_from_nested_meta<'meta, T>(
    meta: &'meta util::Meta,
    attr_name: &str,
    expected_format: &str,
) -> Result<&'meta T>
    where &'meta syn::Lit: util::TryInto<&'meta T>
{
    parse_attr_from_nested_meta_impl(meta, attr_name, expected_format)
        .and_then(|lit| {
            let span = lit.span();
            lit.try_into().ok_or_else(move || wrong_format(span, expected_format))
        })
}

/// Parses required inner attribute from `#[event(...)]` outer attribute.
fn parse_attr_from_nested_meta_impl<'meta>(
    meta: &'meta util::Meta,
    attr_name: &str,
    expected_format: &str,
) -> Result<&'meta syn::Lit> {
    let mut attr = None;

    for meta in meta {
        let meta = match meta {
            syn::NestedMeta::Meta(meta) => meta,
            _ => return Err(wrong_format(meta, expected_format)),
        };

        let meta = match meta {
            syn::Meta::NameValue(meta) => meta,
            _ => return Err(wrong_format(meta, expected_format)),
        };

        if !VALID_ATTR_ARGS.iter().any(|attr| meta.path.is_ident(attr)) {
            return Err(Error::new(meta.span(), "Invalid attribute"));
        }

        if meta.path.is_ident(attr_name) && attr.replace(&meta.lit).is_some() {
            return Err(Error::new(
                meta.span(),
                format!("Only one #[event({})] attribute is allowed", expected_format),
            ));
        }
    }

    attr.ok_or_else(|| {
        Error::new(
            proc_macro2::Span::call_site(),
            format!("Expected to have #[event({})] attribute", expected_format),
        )
    })
}

/// Constructs error message about wrong attribute format.
fn wrong_format<S>(span: S, expected_format: &str) -> Error
    where S: Spanned
{
    Error::new(
        span.span(),
        format!("Wrong attribute format; expected #[event({})]", expected_format),
    )
}
