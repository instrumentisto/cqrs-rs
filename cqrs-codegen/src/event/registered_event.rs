//! Codegen for [`cqrs::RegisteredEvent`].

use proc_macro2::TokenStream;
use quote::quote;
use syn::Result;
use synstructure::Structure;

use crate::util;

/// Name of the derived trait.
const TRAIT_NAME: &str = "RegisteredEvent";

/// Implements [`crate::derive_registered_event`] macro expansion.
pub fn derive(input: syn::DeriveInput) -> Result<TokenStream> {
    util::derive(input, TRAIT_NAME, derive_struct, derive_enum)
}

/// Implements [`crate::derive_registered_event`] macro expansion for structs.
fn derive_struct(input: syn::DeriveInput) -> Result<proc_macro2::TokenStream> {
    let body = quote! {
        #[inline(always)]
        fn type_id(&self) -> ::core::any::TypeId {
            ::core::any::TypeId::of::<Self>()
        }
    };

    super::render_struct(&input, quote!(::cqrs::RegisteredEvent), body, None)
}

/// Implements [`crate::derive_registered_event`] macro expansion for enums
/// via [`synstructure`].
fn derive_enum(input: syn::DeriveInput) -> Result<proc_macro2::TokenStream> {
    util::assert_attr_does_not_exist(&input.attrs, super::ATTR_NAME)?;

    let mut structure = Structure::try_new(&input)?;

    super::assert_all_enum_variants_have_single_field(&structure, TRAIT_NAME)?;

    structure.add_bounds(synstructure::AddBounds::None);

    structure.bind_with(|_| synstructure::BindStyle::Move);
    structure.binding_name(|_, _| syn::Ident::new("_", proc_macro2::Span::call_site()));

    let body = structure.each(|bi| {
        let ty = &bi.ast().ty;
        quote!(::core::any::TypeId::of::<#ty>())
    });

    let mut where_clause = None;

    structure.add_trait_bounds(
        &syn::parse2(quote!(::cqrs::Event))?,
        &mut where_clause,
        synstructure::AddBounds::Fields,
    );

    if let Some(where_clause) = &mut where_clause {
        for predicate in where_clause.predicates.iter_mut() {
            match predicate {
                syn::WherePredicate::Type(predicate) => {
                    predicate.bounds.push(syn::parse2(quote!('static))?);
                }
                _ => (),
            }
        }
    }

    super::render_enum(
        &mut structure,
        quote!(::cqrs::RegisteredEvent),
        quote!(type_id),
        quote!(::core::any::TypeId),
        body,
        where_clause,
    )
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn derives_struct_impl() {
        let input = syn::parse_quote! {
            struct Event;
        };

        let output = quote! {
            #[automatically_derived]
            impl ::cqrs::RegisteredEvent for Event {
                #[inline(always)]
                fn type_id (&self) -> ::core::any::TypeId {
                    ::core::any::TypeId::of::<Self>()
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
            #[allow(non_upper_case_globals)]
            const _DERIVE_cqrs_RegisteredEvent_FOR_Event: () = {
                #[automatically_derived]
                impl ::cqrs::RegisteredEvent for Event {
                    fn type_id(&self) -> ::core::any::TypeId {
                        match *self {
                            Event::Event1(_,) => {{ ::core::any::TypeId::of::<Event1>() }}
                            Event::Event2{other_event: _,} => {{ ::core::any::TypeId::of::<Event2>() }}
                        }
                    }
                }
            };
        };

        assert_eq!(derive(input).unwrap().to_string(), output.to_string())
    }
}
