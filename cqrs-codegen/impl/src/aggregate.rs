//! Codegen for [`cqrs::Aggregate`].

use proc_macro2::TokenStream;
use quote::quote;
use syn::{parse_quote, spanned::Spanned as _, Error, Result};

use crate::util;

/// Name of the derived trait.
const TRAIT_NAME: &str = "Aggregate";

/// Name of the attribute, used by [`cqrs::Aggregate`].
const ATTR_NAME: &str = "aggregate";

/// Implements [`crate::aggregate_derive`] macro expansion.
pub fn derive(input: syn::DeriveInput) -> Result<TokenStream> {
    util::derive(input, TRAIT_NAME, derive_struct, derive_enum)
}

/// Implements [`crate::aggregate_derive`] macro expansion for structs.
fn derive_struct(input: syn::DeriveInput) -> Result<TokenStream> {
    let meta = util::get_nested_meta(&input.attrs, ATTR_NAME)?;

    let data = match &input.data {
        syn::Data::Struct(data) => data,
        _ => unreachable!(),
    };

    let const_val = parse_aggregate_type(&meta)?;
    let const_doc = format!("Type name of [`{}`] aggregate", input.ident);

    let (id_type, id_field) = get_id_field(&data.fields)?;

    let type_name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let mut extended_generics = input.generics.clone();
    extended_generics
        .make_where_clause()
        .predicates
        .push(parse_quote! { Self: ::core::default::Default });
    let (_, _, extended_where_clause) = extended_generics.split_for_impl();

    Ok(quote! {
        #[automatically_derived]
        impl#impl_generics #type_name#ty_generics #where_clause {
            #[doc = #const_doc]
            pub const AGGREGATE_TYPE: ::cqrs::AggregateType = #const_val;
        }

        #[automatically_derived]
        impl#impl_generics ::cqrs::Aggregate for #type_name#ty_generics #extended_where_clause {
            type Id = #id_type;

            #[inline(always)]
            fn aggregate_type(&self) -> ::cqrs::AggregateType {
                Self::AGGREGATE_TYPE
            }

            #[inline(always)]
            fn id(&self) -> &Self::Id {
                &self.#id_field
            }
        }
    })
}

/// Reports error if [`crate::aggregate_derive`] macro applied to enums.
fn derive_enum(input: syn::DeriveInput) -> Result<TokenStream> {
    match input.data {
        syn::Data::Enum(data) => Err(Error::new(
            data.enum_token.span(),
            format!("Structs are not supported for deriving {}", TRAIT_NAME),
        )),
        _ => unreachable!(),
    }
}

/// Parses type of [`cqrs::Aggregate`] from `#[aggregate(...)]` attribute.
fn parse_aggregate_type(meta: &util::Meta) -> Result<String> {
    let lit: &syn::LitStr = util::parse_lit(meta, "name", &["name"], ATTR_NAME, "= \"...\"")?;

    Ok(lit.value())
}

/// Infers or finds via `#[aggregate(id)]` attribute an `id` field
/// of this aggregate.
fn get_id_field(fields: &syn::Fields) -> Result<(&syn::Type, TokenStream)> {
    let mut id = util::find_field_with_flag(fields, ATTR_NAME, "id", &["id"])?;

    let is_named = match fields {
        syn::Fields::Named(_) => true,
        _ => false,
    };

    if id.is_none() && is_named {
        id = fields.iter().enumerate().find(|(_, f)| match &f.ident {
            Some(ident) => ident == "id",
            None => false,
        });
    }

    id.map(|(index, field)| {
        let ty = &field.ty;
        let ident = util::render_field_ident(index, field);
        (ty, ident)
    })
    .ok_or_else(|| Error::new(fields.span(), "No 'id' field found for an aggregate"))
}

#[cfg(test)]
mod spec {
    use super::*;

    #[test]
    fn derives_struct_impl() {
        let input = syn::parse_quote! {
            #[aggregate(name = "aggregate")]
            struct Aggregate {
                id: AggregateId,
                field: i32,
            }
        };

        let output = quote! {
            #[automatically_derived]
            impl Aggregate {
                #[doc = "Type name of [`Aggregate`] aggregate"]
                pub const AGGREGATE_TYPE: ::cqrs::AggregateType = "aggregate";
            }

            #[automatically_derived]
            impl ::cqrs::Aggregate for Aggregate {
                type Id = AggregateId;

                #[inline(always)]
                fn aggregate_type(&self) -> ::cqrs::AggregateType {
                    Self::AGGREGATE_TYPE
                }

                #[inline(always)]
                fn id(&self) -> &Self::Id {
                    &self.id
                }
            }
        };

        assert_eq!(derive(input).unwrap().to_string(), output.to_string());
    }
}
