//! Codegen for [`cqrs::TypedEvent`].

use proc_macro2::TokenStream;
use quote::quote;
use syn::{parse_quote, spanned::Spanned as _, Error, Result};
use synstructure::Structure;

use crate::util;

/// Name of the derived trait.
const TRAIT_NAME: &str = "TypedEvent";

/// Implements `cqrs::TypedEvent` part of [`crate::event_derive`] macro
/// expansion.
pub fn derive(input: syn::DeriveInput) -> Result<TokenStream> {
    util::derive(input, TRAIT_NAME, derive_struct, derive_enum)
}

/// Implements `cqrs::TypedEvent` part of [`crate::event_derive`] macro
/// expansion for structs.
fn derive_struct(input: syn::DeriveInput) -> Result<TokenStream> {
    let type_name = &input.ident;
    let (impl_gens, ty_gens, where_clause) = input.generics.split_for_impl();

    let mut where_clause = where_clause.cloned().unwrap_or_else(|| parse_quote!(where));
    where_clause
        .predicates
        .push(parse_quote!(Self: ::cqrs::StaticTypedEvent));

    let const_doc = format!("Type names of [`{type_name}`] events.");

    Ok(quote! {
        #[automatically_derived]
        impl#impl_gens ::cqrs::TypedEvent for #type_name#ty_gens #where_clause {
            #[doc = #const_doc]
            const EVENT_TYPES: &'static [::cqrs::EventType] = &[
                <Self as ::cqrs::StaticTypedEvent>::EVENT_TYPE
            ];
        }
    })
}

/// Implements `cqrs::TypedEvent` part of [`crate::event_derive`] macro
/// expansion for enums via [`synstructure`].
fn derive_enum(input: syn::DeriveInput) -> Result<TokenStream> {
    let structure = Structure::try_new(&input)?;
    util::assert_all_enum_variants_have_single_field(&structure, TRAIT_NAME)?;

    let iter = structure
        .variants()
        .iter()
        .map(|variant| variant.ast().fields.iter())
        .flatten();

    let mut types = Vec::new();
    for field in iter {
        let mut path = match &field.ty {
            syn::Type::Path(path) => path.path.clone(),
            _ => {
                return Err(Error::new(
                    field.span(),
                    "TypedEvent can only be derived for enums \
                     with variants containing owned scalar data",
                ))
            }
        };

        // type-path cannot ever be empty, unless there is an error in syn
        let first_segment = path.segments.first().unwrap();

        let is_generic = input.generics.params.iter().any(|p| match p {
            syn::GenericParam::Type(p) => p.ident == first_segment.ident,
            syn::GenericParam::Const(_) | syn::GenericParam::Lifetime(_) => false,
        });
        if is_generic {
            return Err(Error::new(
                first_segment.ident.span(),
                "Type parameters are not allowed here, as they cannot have \
                 associated constants (but generic types dependent on generic \
                 type parameters, e.g., 'Event<T>', are fine)",
            ));
        }

        // type-path cannot ever be empty, unless there is an error in syn
        let last_segment = path.segments.last_mut().unwrap();

        if let syn::PathArguments::AngleBracketed(args) = &mut last_segment.arguments {
            args.colon2_token = Some(Default::default());
        }

        types.push(path);
    }

    let type_name = &input.ident;
    let const_doc = format!("Type names of [`{type_name}`] events.");

    let mut where_clause = input
        .generics
        .where_clause
        .clone()
        .unwrap_or_else(|| parse_quote!(where));
    for ty in &types {
        where_clause
            .predicates
            .push(parse_quote!(#ty: ::cqrs::TypedEvent));
    }

    let (impl_generics, ty_generics, _) = input.generics.split_for_impl();

    let subtypes = types
        .iter()
        .map(|ty| quote! { <#ty as ::cqrs::TypedEvent>::EVENT_TYPES })
        .collect::<Vec<_>>();

    let len = quote! {
        0 #(+ #subtypes.len())*
    };

    Ok(quote! {
        #[automatically_derived]
        impl#impl_generics ::cqrs::TypedEvent for #type_name#ty_generics #where_clause {
            #[doc = #const_doc]
            const EVENT_TYPES: &'static [::cqrs::EventType] = {
                ::cqrs::private::slice_arr(
                    &const {
                        const __LEN: usize = 128;
                        if #len > __LEN {
                            panic!("`cqrs::TypedEvent::EVENT_TYPES` limit reached");
                        }

                        let mut out = [""; __LEN];
                        let mut len = 0;

                        #({
                            let mut i = 0;
                            while i < #subtypes.len() {
                                out[len] = #subtypes[i];
                                i += 1;
                                len += 1;
                            }
                        })*

                        out
                    },
                    #len,
                )
            };
        }
    })
}

#[cfg(test)]
mod spec {
    use super::*;

    #[test]
    fn derives_struct_impl() {
        let input = syn::parse_quote! {
            #[event(name = "event")]
            struct Event;
        };

        let output = quote! {
            #[automatically_derived]
            impl ::cqrs::TypedEvent for Event
            where
                Self: ::cqrs::StaticTypedEvent
            {
                #[doc = "Type names of [`Event`] events."]
                const EVENT_TYPES: &'static [::cqrs::EventType] = &[
                    <Self as ::cqrs::StaticTypedEvent>::EVENT_TYPE
                ];
            }
        };

        assert_eq!(derive(input).unwrap().to_string(), output.to_string())
    }

    #[test]
    fn derives_enum_impl() {
        let input = syn::parse_quote! {
            enum Event {
                MyEvent(MyEvent),
                HisEvent(HisEvent),
                HerEvent(HerEvent),
            }
        };

        let output = quote! {
            #[automatically_derived]
            impl ::cqrs::TypedEvent for Event
            where
                MyEvent: ::cqrs::TypedEvent,
                HisEvent: ::cqrs::TypedEvent,
                HerEvent: ::cqrs::TypedEvent
            {
                #[doc = "Type names of [`Event`] events."]
                const EVENT_TYPES: &'static [::cqrs::EventType] = {
                    ::cqrs::const_concat_slices!(
                        ::cqrs::EventType,
                        <MyEvent as ::cqrs::TypedEvent>::EVENT_TYPES,
                        <HisEvent as ::cqrs::TypedEvent>::EVENT_TYPES,
                        <HerEvent as ::cqrs::TypedEvent>::EVENT_TYPES
                    )
                };
            }
        };

        assert_eq!(derive(input).unwrap().to_string(), output.to_string())
    }
}
