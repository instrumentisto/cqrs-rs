//! Codegen for [`cqrs::Event`]

use quote::quote;
use syn::{
    parse::{Error, Result},
    punctuated::Punctuated,
    spanned::Spanned as _,
};
use synstructure::Structure;

use crate::{event, util};

/// Implements [`crate::derive_event`] macro expansion.
pub fn derive(input: syn::DeriveInput) -> Result<proc_macro2::TokenStream> {
    match input.data {
        syn::Data::Struct(_) => derive_struct(input),
        syn::Data::Enum(_) => derive_enum(input),
        syn::Data::Union(data) => Err(Error::new(
            data.union_token.span(),
            "Unions are not supported for deriving Event",
        )),
    }
}

/// Implements [`crate::derive_event`] macro expansion for structs.
pub fn derive_struct(input: syn::DeriveInput) -> Result<proc_macro2::TokenStream> {
    let syn::DeriveInput {
        attrs,
        ident,
        generics,
        ..
    } = input;

    let meta = util::get_nested_meta(&attrs, "event")?.ok_or_else(|| {
        Error::new(
            ident.span(),
            "Expected struct to have #[event(...)] attribute",
        )
    })?;

    let const_val = parse_event_type_from_nested_meta(&meta)?;
    let const_doc = format!("Type name of [`{}`] event", ident);

    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    Ok(quote! {
        #[automatically_derived]
        impl#impl_generics #ident#ty_generics #where_clause {
            #[doc = #const_doc]
            pub const EVENT_TYPE: ::cqrs::EventType = #const_val;
        }

        #[automatically_derived]
        impl#impl_generics ::cqrs::Event for #ident#ty_generics #where_clause {
            #[inline(always)]
            fn event_type(&self) -> ::cqrs::EventType {
                Self::EVENT_TYPE
            }
        }
    })
}

/// Implements [`crate::derive_event`] macro expansion for enums.
fn derive_enum(input: syn::DeriveInput) -> Result<proc_macro2::TokenStream> {
    derive_enum_impl(Structure::try_new(&input)?)
}

/// Implements [`crate::derive_event`] macro expansion for enums
/// via [`crate::event::common::derive_enum_impl`] and [`synstructure`].
fn derive_enum_impl(mut structure: Structure) -> Result<proc_macro2::TokenStream> {
    let body = event::common::derive_enum_impl(&mut structure, "Event", "event_type")?;

    Ok(structure.gen_impl(quote! {
        #[automatically_derived]
        gen impl ::cqrs::Event for @Self {
            fn event_type(&self) -> ::cqrs::EventType {
                match *self {
                    #body
                }
            }
        }
    }))
}

/// Parses type of [`cqrs::Event`] from `#[event(...)]` attribute.
fn parse_event_type_from_nested_meta(
    meta: &Punctuated<syn::NestedMeta, syn::Token![,]>,
) -> Result<String> {
    const EXPECTED_FORMAT: &str = "type = \"...\"";

    let lit = event::common::parse_attr_from_nested_meta(meta, "type", EXPECTED_FORMAT)?;

    let event_type = match lit {
        syn::Lit::Str(lit) => lit.value(),
        _ => {
            return Err(Error::new(
                lit.span(),
                event::common::wrong_format(EXPECTED_FORMAT),
            ))
        }
    };

    Ok(event_type)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn derives_struct_impl() {
        let input = syn::parse_quote! {
            #[event(type = "event")]
            struct Event;
        };

        let output = derive_struct(input).unwrap();

        let expected_output = quote! {
            #[automatically_derived]
            impl Event {
                #[doc = "Type name of [`Event`] event"]
                pub const EVENT_TYPE: ::cqrs::EventType = "event";
            }

            #[automatically_derived]
            impl ::cqrs::Event for Event {
                #[inline(always)]
                fn event_type(&self) -> ::cqrs::EventType {
                    Self::EVENT_TYPE
                }
            }
        };

        assert_eq!(output.to_string(), expected_output.to_string());
    }

    #[test]
    fn derives_enum_impl() {
        synstructure::test_derive! {
            derive_enum_impl {
                enum Event {
                    Event1(Event1),
                    Event2 {
                        other_event: Event2,
                    },
                }
            }
            expands to {
                #[allow(non_upper_case_globals)]
                const _DERIVE_cqrs_Event_FOR_Event: () = {
                    #[automatically_derived]
                    impl ::cqrs::Event for Event {
                        fn event_type(&self) -> ::cqrs::EventType {
                            match *self {
                                Event::Event1(ref event,) => {{ event.event_type() }}
                                Event::Event2{other_event: ref other_event,} => {{ other_event.event_type() }}
                            }
                        }
                    }
                };
            }
            no_build
        }
    }
}
