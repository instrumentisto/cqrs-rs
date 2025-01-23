//! Codegen for [`cqrs::Event`].

use proc_macro2::TokenStream;
use quote::quote;
use syn::{parse_quote, spanned::Spanned as _, Error, Result};
use synstructure::Structure;

use crate::{event::typed_event, util};

/// Name of the derived trait.
const TRAIT_NAME: &str = "Event";

/// Implements [`crate::event_derive`] macro expansion.
pub fn derive(input: syn::DeriveInput) -> Result<TokenStream> {
    let mut s = util::derive(input.clone(), TRAIT_NAME, derive_struct, derive_enum)?;
    s.extend(typed_event::derive(input)?);
    panic!("{}", s);
    Ok(s)
}

/// Implements [`crate::event_derive`] macro expansion for structs.
fn derive_struct(input: syn::DeriveInput) -> Result<TokenStream> {
    let meta = util::get_nested_meta(&input.attrs, super::ATTR_NAME)?;

    let const_val = parse_event_type_from_nested_meta(&meta)?;
    let const_doc = format!("Type name of [`{}`] event.", input.ident);

    let type_name = &input.ident;
    let (impl_gens, ty_gens, type_where_clause) = input.generics.split_for_impl();

    let mut event_where_clause = type_where_clause
        .cloned()
        .unwrap_or_else(|| parse_quote!(where));
    event_where_clause
        .predicates
        .push(parse_quote!(Self: ::cqrs::StaticTypedEvent));

    Ok(quote! {
        #[automatically_derived]
        impl#impl_gens ::cqrs::StaticTypedEvent for #type_name#ty_gens
        #type_where_clause
        {
            #[doc = #const_doc]
            const EVENT_TYPE: ::cqrs::EventType = #const_val;
        }

        #[automatically_derived]
        impl#impl_gens ::cqrs::Event for #type_name#ty_gens
        #event_where_clause
        {
            #[inline(always)]
            fn event_type(&self) -> ::cqrs::EventType {
                <Self as ::cqrs::StaticTypedEvent>::EVENT_TYPE
            }
        }
    })
}

/// Implements [`crate::event_derive`] macro expansion for enums
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
            .push(parse_quote!(#ty: ::cqrs::Event));
    }

    let variant = data.variants.iter().map(|v| {
        let ident = &v.ident;
        let field = &v.fields.iter().next().expect("already checked");
        if field.ident.is_some() {
            quote! { Self::#ident { #field: ref ev } => ev.event_type() }
        } else {
            quote! { Self::#ident(ref ev) => ev.event_type() }
        }
    });

    let (impl_generics, ty_generics, _) = input.generics.split_for_impl();

    Ok(quote! {
        #[automatically_derived]
        impl #impl_generics ::cqrs::Event for #type_name #ty_generics
        #where_clause
        {
            fn event_type(&self) -> ::cqrs::EventType {
                match *self {
                    #( #variant, )*
                }
            }
        }
    })
}

/// Parses type of [`cqrs::Event`] from `#[event(...)]` attribute.
fn parse_event_type_from_nested_meta(meta: &util::Meta) -> Result<String> {
    let lit: &syn::LitStr = util::parse_lit(
        meta,
        "name",
        super::VALID_STRUCT_ARGS,
        super::ATTR_NAME,
        "= \"...\"",
    )?;
    Ok(lit.value())
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
            impl ::cqrs::StaticTypedEvent for Event {
                #[doc = "Type name of [`Event`] event."]
                const EVENT_TYPE: ::cqrs::EventType = "event";
            }
            #[automatically_derived]
            impl ::cqrs::Event for Event
            where
                Self: ::cqrs::StaticTypedEvent
            {
                #[inline(always)]
                fn event_type(&self) -> ::cqrs::EventType {
                    <Self as ::cqrs::StaticTypedEvent>::EVENT_TYPE
                }
            }
            #[automatically_derived]
            impl ::cqrs::TypedEvent for Event
            where
                Self: ::cqrs::StaticTypedEvent
            {
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
                Event1(Event1),
                Event2 {
                    other_event: Event2,
                },
            }
        };

        let output = quote! {
            const _: () = {
                #[automatically_derived]
                impl ::cqrs::Event for Event
                where
                    Event1: ::cqrs::Event,
                    Event2: ::cqrs::Event
                {
                    fn event_type(&self) -> ::cqrs::EventType {
                        match *self {
                            Event::Event1(ref ev,) => {{ ev.event_type() }}
                            Event::Event2{other_event: ref other_event,} => {{ other_event.event_type() }}
                        }
                    }
                }
            };
            #[automatically_derived]
            impl ::cqrs::TypedEvent for Event
            where Event1: ::cqrs::TypedEvent,
                  Event2: ::cqrs::TypedEvent
            {
                #[doc = "Type names of [`Event`] events."]
                const EVENT_TYPES: &'static [::cqrs::EventType] = {
                    ::cqrs::private::slice_arr(
                        &const {
                            const __LEN: usize = 128;
                            if 0
                                + <Event1 as ::cqrs::TypedEvent>::EVENT_TYPES
                                     .len()
                                + <Event2 as ::cqrs::TypedEvent>::EVENT_TYPES
                                     .len()
                                > __LEN {
                                panic!("`cqrs::TypedEvent::EVENT_TYPES` limit reached");
                            }

                            let mut out = [""; __LEN];
                            let mut len = 0;

                            let mut i = 0;
                            while i < <Event1 as ::cqrs::TypedEvent>::EVENT_TYPES.len() {
                                out[len] = <Event1 as ::cqrs::TypedEvent>::EVENT_TYPES[i];
                                i += 1;
                                len += 1;
                            }

                            let mut i = 0;
                            while i < <Event2 as ::cqrs::TypedEvent>::EVENT_TYPES.len() {
                                out[len] = <Event2 as ::cqrs::TypedEvent>::EVENT_TYPES[i];
                                i += 1;
                                len += 1;
                            }

                            out
                        },
                        0 + <Event1 as ::cqrs::TypedEvent>::EVENT_TYPES.len()
                          + <Event2 as ::cqrs::TypedEvent>::EVENT_TYPES.len(),
                    )
                };
            }
        };

        assert_eq!(derive(input).unwrap().to_string(), output.to_string())
    }
}
