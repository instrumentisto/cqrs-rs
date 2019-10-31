//! Shared implementation details for [`cqrs::Event`] proc-macro-derive family

use quote::quote;
use syn::{
    parse::{Error, Result},
    punctuated::Punctuated,
    spanned::Spanned as _,
};
use synstructure::Structure;

use crate::util;

pub const OUTER_ATTR_NAME: &str = "event";

pub const INNER_ATTR_NAMES: &[&str] = &["type", "version"];

/// Implements macro expansion for enums for [`cqrs::Event`] proc-macro-derive family.
pub fn derive_enum_impl(
    structure: &mut Structure,
    trait_name: &str,
    method_name: &str,
) -> Result<proc_macro2::TokenStream> {
    if util::get_nested_meta(&structure.ast().attrs, OUTER_ATTR_NAME)?.is_some() {
        return Err(Error::new(
            structure.ast().span(),
            "#[event(...)] attribute is not allowed for enums",
        ));
    }

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

    structure.add_bounds(synstructure::AddBounds::Fields);

    structure.binding_name(|field, _| {
        field.ident.as_ref().map_or_else(
            || syn::Ident::new("event", proc_macro2::Span::call_site()),
            |ident| ident.clone(),
        )
    });

    let method_name = syn::Ident::new(method_name, proc_macro2::Span::call_site());

    let body = structure.each(|binding_info| {
        let ident = &binding_info.binding;
        quote!(#ident.#method_name())
    });

    Ok(body)
}

/// Parses required inner attribute from `#[event(...)]` outer attribute.
pub fn parse_attr_from_nested_meta<'meta>(
    meta: &'meta Punctuated<syn::NestedMeta, syn::Token![,]>,
    attr_name: &str,
    expected_format: &str,
) -> Result<&'meta syn::Lit> {
    let mut attr = None;

    for meta in meta {
        let meta = match meta {
            syn::NestedMeta::Meta(meta) => meta,
            _ => return Err(Error::new(meta.span(), wrong_format(expected_format))),
        };

        let meta = match meta {
            syn::Meta::NameValue(meta) => meta,
            _ => return Err(Error::new(meta.span(), wrong_format(expected_format))),
        };

        if !INNER_ATTR_NAMES.iter().any(|attr| meta.path.is_ident(attr)) {
            return Err(Error::new(meta.span(), "Invalid attribute"));
        }

        if meta.path.is_ident(attr_name) && attr.replace(&meta.lit).is_some() {
            return Err(Error::new(
                meta.span(),
                format!(
                    "Only one #[event({})] attribute is allowed",
                    expected_format
                ),
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

/// Returns "Wrong attribute format" error message.
pub fn wrong_format(expected_format: &str) -> String {
    format!(
        "Wrong attribute format; expected #[event({})]",
        expected_format
    )
}
