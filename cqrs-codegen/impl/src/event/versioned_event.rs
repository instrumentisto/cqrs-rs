//! Codegen for [`cqrs::VersionedEvent`].

use std::num::NonZeroU8;

use proc_macro2::TokenStream;
use quote::quote;
use syn::{parse_quote, Result};
use synstructure::Structure;

use crate::util;

/// Name of the derived trait.
const TRAIT_NAME: &str = "VersionedEvent";

/// Implements [`crate::versioned_event_derive`] macro expansion.
pub fn derive(input: syn::DeriveInput) -> Result<TokenStream> {
    util::derive(input, TRAIT_NAME, derive_struct, derive_enum)
}

/// Implements [`crate::versioned_event_derive`] macro expansion for structs.
fn derive_struct(input: syn::DeriveInput) -> Result<TokenStream> {
    let meta = util::get_nested_meta(&input.attrs, super::ATTR_NAME)?;

    let type_name = &input.ident;
    let (impl_gens, ty_gens, ver_where_clause) = input.generics.split_for_impl();

    let mut event_where_clause = ver_where_clause
        .cloned()
        .unwrap_or_else(|| parse_quote!(where));
    event_where_clause
        .predicates
        .push(parse_quote!(Self: ::cqrs::StaticVersionedEvent));

    let const_val = parse_event_version_from_nested_meta(&meta)?;
    let const_doc = format!("Version of [`{type_name}`] event");

    Ok(quote! {
        #[automatically_derived]
        impl#impl_gens ::cqrs::StaticVersionedEvent for #type_name#ty_gens
        #ver_where_clause
        {
            #[doc = #const_doc]
            const EVENT_VERSION: ::cqrs::EventVersion =
                unsafe { ::cqrs::EventVersion::new_unchecked(#const_val) };
        }

        #[automatically_derived]
        impl#impl_gens ::cqrs::VersionedEvent for #type_name#ty_gens
        #event_where_clause
        {
            #[inline(always)]
            fn event_version(&self) -> &'static ::cqrs::EventVersion {
                &<Self as ::cqrs::StaticVersionedEvent>::EVENT_VERSION
            }
        }
    })
}

/// Implements [`crate::versioned_event_derive`] macro expansion for enums
/// via [`synstructure`].
fn derive_enum(input: syn::DeriveInput) -> Result<TokenStream> {
    util::assert_valid_attr_args_used(&input.attrs, super::ATTR_NAME, super::VALID_ENUM_ARGS)?;

    let structure = Structure::try_new(&input)?;
    util::assert_all_enum_variants_have_single_field(&structure, TRAIT_NAME)?;

    let syn::Data::Enum(data) = input.data else {
        unreachable!("already checked")
    };

    let type_name = &input.ident;

    let mut where_clause = input
        .generics
        .where_clause
        .clone()
        .unwrap_or_else(|| parse_quote!(where));
    for v in &data.variants {
        let ty = &v.fields.iter().next().expect("already checked").ty;
        where_clause
            .predicates
            .push(parse_quote!(#ty: ::cqrs::VersionedEvent));
    }

    let variant = data.variants.iter().map(|v| {
        let ident = &v.ident;
        let field = &v.fields.iter().next().expect("already checked");
        if let Some(field_ident) = &field.ident {
            quote! { Self::#ident { #field_ident: ref ev } => ev.event_version() }
        } else {
            quote! { Self::#ident(ref ev) => ev.event_version() }
        }
    });

    let (impl_generics, ty_generics, _) = input.generics.split_for_impl();

    Ok(quote! {
        #[automatically_derived]
        impl #impl_generics ::cqrs::VersionedEvent for #type_name #ty_generics
        #where_clause
        {
            fn event_version(&self) -> &'static ::cqrs::EventVersion {
                match *self {
                    #( #variant, )*
                }
            }
        }
    })
}

/// Parses version of [`cqrs::Event`] from `#[event(...)]` attribute.
fn parse_event_version_from_nested_meta(meta: &util::Meta) -> Result<u8> {
    let lit: &syn::LitInt = util::parse_lit(
        meta,
        "version",
        super::VALID_STRUCT_ARGS,
        super::ATTR_NAME,
        "= <non-zero unsigned integer>",
    )?;
    Ok(lit.base10_parse::<NonZeroU8>()?.get())
}

#[cfg(test)]
mod spec {
    use super::*;

    #[test]
    fn derives_struct_impl() {
        let input = syn::parse_quote! {
            #[event(version = 1)]
            struct Event;
        };

        let output = quote! {
            #[automatically_derived]
            impl ::cqrs::StaticVersionedEvent for Event {
                #[doc = "Version of [`Event`] event"]
                pub const EVENT_VERSION: ::cqrs::EventVersion =
                    unsafe { ::cqrs::EventVersion::new_unchecked(1u8) };
            }

            #[automatically_derived]
            impl ::cqrs::VersionedEvent for Event
            where
                Self: ::cqrs::StaticVersionedEvent
            {
                #[inline(always)]
                fn event_version(&self) -> &'static ::cqrs::EventVersion {
                    &<Self as ::cqrs::StaticVersionedEvent>::EVENT_VERSION
                }
            }
        };

        assert_eq!(derive(input).unwrap().to_string(), output.to_string())
    }

    #[test]
    fn derives_enum_impl() {
        let input = syn::parse_quote! {
            enum Event {
                Event1(Event1),
                Event2 {
                    other_event: Event2,
                },
            }
        };

        let output = quote! {
            #[automatically_derived]
            impl ::cqrs::VersionedEvent for Event
            where
                Event1: ::cqrs::VersionedEvent,
                Event2: ::cqrs::VersionedEvent
            {
                fn event_version(&self) -> &'static ::cqrs::EventVersion {
                    match *self {
                        Event::Event1(ref ev,) => {{ ev.event_version() }}
                        Event::Event2{other_event: ref other_event,} => {{ other_event.event_version() }}
                    }
                }
            }
        };

        assert_eq!(derive(input).unwrap().to_string(), output.to_string())
    }
}
