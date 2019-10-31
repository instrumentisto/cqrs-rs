use quote::quote;
use syn::{
    parse::{Error, Result},
    punctuated::Punctuated,
    spanned::Spanned as _,
};
use synstructure::Structure;

use crate::{event, util};

/// Implements [`crate::derive_versioned_event`] macro expansion.
pub fn derive(input: syn::DeriveInput) -> Result<proc_macro2::TokenStream> {
    match input.data {
        syn::Data::Struct(_) => derive_struct(input),
        syn::Data::Enum(_) => derive_enum(input),
        syn::Data::Union(data) => Err(Error::new(
            data.union_token.span(),
            "Unions are not supported for deriving VersionedEvent",
        )),
    }
}

/// Implements [`crate::derive_versioned_event`] macro expansion for structs.
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

    let const_val = parse_event_version_from_nested_meta(&meta)?;
    let const_doc = format!("Version of [`{}`] event", ident);

    let (
        impl_generics,
        ty_generics,
        where_clause
    ) = generics.split_for_impl();

    Ok(quote! {
        #[automatically_derived]
        impl#impl_generics #ident#ty_generics #where_clause {
            #[doc = #const_doc]
            #[allow(unsafe_code)]
            pub const EVENT_VERSION: ::cqrs::EventVersion =
                unsafe { ::cqrs::EventVersion::new_unchecked(#const_val) };
        }

        #[automatically_derived]
        impl#impl_generics ::cqrs::VersionedEvent for #ident#ty_generics #where_clause {
            fn event_version(&self) -> &'static ::cqrs::EventVersion {
                &Self::EVENT_VERSION
            }
        }
    })
}

/// Implements [`crate::derive_versioned_event`] macro expansion for enums.
fn derive_enum(input: syn::DeriveInput) -> Result<proc_macro2::TokenStream> {
    derive_enum_impl(Structure::try_new(&input)?)
}

/// Implements [`crate::derive_versioned_event`] macro expansion for enums
/// via [`synstructure`].
fn derive_enum_impl(mut structure: Structure) -> Result<proc_macro2::TokenStream> {
    let body = event::common::derive_enum_impl(
        &mut structure,
        "VersionedEvent",
        "event_version"
    )?;

    Ok(structure.gen_impl(quote! {
        #[automatically_derived]
        gen impl ::cqrs::VersionedEvent for @Self {
            fn event_version(&self) -> &'static ::cqrs::EventVersion {
                match *self {
                    #body
                }
            }
        }
    }))
}

/// Parses type of [`cqrs::Event`] from `#[event(...)]` attribute.
fn parse_event_version_from_nested_meta(
    meta: &Punctuated<syn::NestedMeta, syn::Token![,]>,
) -> Result<u8> {
    const EXPECTED_FORMAT: &str = "version = <non-zero unsigned integer>";

    let lit = event::common::parse_attr_from_nested_meta(
        meta,
        "version",
        EXPECTED_FORMAT
    )?;

    let event_version = match lit {
        syn::Lit::Int(lit) => lit.base10_parse::<std::num::NonZeroU8>()?.get(), // TODO
        _ => return Err(Error::new(
            lit.span(),
            event::common::wrong_format(EXPECTED_FORMAT)
        )),
    };

    Ok(event_version)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn derives_struct_impl() {
        let input = syn::parse_quote! {
            #[event(version = 1)]
            struct Event;
        };

        let output = derive_struct(input).unwrap();

        let expected_output = quote! {
            #[automatically_derived]
            impl Event {
                #[doc = "Version of [`Event`] event"]
                #[allow(unsafe_code)]
                pub const EVENT_VERSION: ::cqrs::EventVersion =
                    unsafe { ::cqrs::EventVersion::new_unchecked(1u8) };
            }

            #[automatically_derived]
            impl ::cqrs::VersionedEvent for Event {
                fn event_version(&self) -> &'static ::cqrs::EventVersion {
                    &Self::EVENT_VERSION
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
                const _DERIVE_cqrs_VersionedEvent_FOR_Event: () = {
                    #[automatically_derived]
                    impl ::cqrs::VersionedEvent for Event {
                        fn event_version(&self) -> &'static ::cqrs::EventVersion {
                            match *self {
                                Event::Event1(ref event,) => {{ event.event_version() }}
                                Event::Event2{other_event: ref other_event,} => {{ other_event.event_version() }}
                            }
                        }
                    }
                };
            }
            no_build
        }
    }
}
