//! Codegen for [`cqrs::Event`] and related traits
//! (e.g. [`cqrs::VersionedEvent`], etc).

mod aggregate_event;
mod event;
mod registered_event;
mod typed_event;
mod versioned_event;

use proc_macro2::TokenStream;
use quote::quote;
use syn::Result;
use synstructure::Structure;

use crate::util;

pub use aggregate_event::derive as aggregate_event_derive;
pub use event::derive as event_derive;
pub use registered_event::derive as registered_event_derive;
pub use versioned_event::derive as versioned_event_derive;

/// Name of the attribute, used for this family of derives.
const ATTR_NAME: &str = "event";

/// Names of the `#[event(...)]` attribute's arguments, used on structs
/// for this family of derives.
const VALID_STRUCT_ARGS: &[&str] = &["name", "version"];

/// Names of the `#[event(...)]` attribute's arguments, used on enums
/// for this family of derives.
const VALID_ENUM_ARGS: &[&str] = &["aggregate"];

/// Renders implementation of a `trait_path` trait as a `method` that proxies
/// call to its variants.
///
/// Expects that all variants of `structure` contain exactly one field.
/// Returns error otherwise.
///
/// `trait_name` is only used to generate error message.
fn render_enum_proxy_method_calls(
    structure: &mut Structure,
    trait_name: &str,
    trait_path: TokenStream,
    method: TokenStream,
    method_return_type: TokenStream,
) -> Result<TokenStream> {
    util::assert_all_enum_variants_have_single_field(&structure, trait_name)?;

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

    Ok(structure.gen_impl(quote! {
        #[automatically_derived]
        gen impl #trait_path for @Self {
            fn #method(&self) -> #method_return_type {
                match *self {
                    #body
                }
            }
        }
    }))
}
