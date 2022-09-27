//! Codegen for [`cqrs::TypedEvent`].

use std::collections::HashSet;

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
    let body = quote! {
        const EVENT_TYPES: &'static [::cqrs::EventType] = &[Self::EVENT_TYPE];
    };

    util::render_struct(&input, quote!(::cqrs::TypedEvent), body, None)
}

/// Implements `cqrs::TypedEvent` part of [`crate::event_derive`] macro
/// expansion for enums via [`synstructure`].
fn derive_enum(input: syn::DeriveInput) -> Result<TokenStream> {
    let structure = Structure::try_new(&input)?;
    util::assert_all_enum_variants_have_single_field(&structure, TRAIT_NAME)?;

    let type_params: HashSet<_> = input
        .generics
        .params
        .iter()
        .filter_map(|generic_param| match generic_param {
            syn::GenericParam::Type(type_param) => Some(&type_param.ident),
            _ => None,
        })
        .collect();

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

        if type_params.contains(&first_segment.ident) {
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

        types.push(quote!(#path));
    }

    let const_doc = format!("Type names of [`{}`] events.", input.ident);

    let type_name = &input.ident;

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
        .map(|ty| quote!(<#ty as ::cqrs::TypedEvent>::EVENT_TYPES))
        .collect::<Vec<_>>();

    let type_params = type_params.into_iter().collect::<Vec<_>>();
    let r = quote! {
        #[automatically_derived]
        impl#impl_generics ::cqrs::TypedEvent for #type_name#ty_generics #where_clause {
            #[doc = #const_doc]
            const EVENT_TYPES: &'static [::cqrs::EventType] = {
                #( type #type_params = (); )*
                ::cqrs::const_concat_slices!(::cqrs::EventType, #( #subtypes ),*)
            };
        }
    };
    // panic!("{}", r);
    Ok(r)
}

#[cfg(test)]
mod spec {
    use super::*;

    #[test]
    fn derives_struct_impl() {
        let input = syn::parse_quote! {
            #[event(type = "event")]
            struct Event;
        };

        let output = quote! {
            #[automatically_derived]
            impl ::cqrs::TypedEvent for Event {
                type EventTypes = std::iter::Once<::cqrs::EventType>;

                #[inline(always)]
                fn event_types() -> Self::EventTypes {
                    std::iter::once(Self::EVENT_TYPE)
                }
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
            impl ::cqrs::TypedEvent for Event {
                type EventTypes = std::iter::Chain<
                    std::iter::Chain<
                        <MyEvent as ::cqrs::TypedEvent>::EventTypes,
                        <HisEvent as ::cqrs::TypedEvent>::EventTypes
                    >,
                    <HerEvent as ::cqrs::TypedEvent>::EventTypes
                >;

                #[inline(always)]
                fn event_types() -> Self::EventTypes {
                    MyEvent::event_types()
                        .chain(HisEvent::event_types())
                        .chain(HerEvent::event_types())
                }
            }
        };

        assert_eq!(derive(input).unwrap().to_string(), output.to_string())
    }
}
