mod event;
mod utility;

extern crate proc_macro;

use proc_macro2;
use syn;

// ...

/// Derives ::cqrs::Event for structs and enums.
///
/// # Deriving for structs
///
/// When deriving ::cqrs::Event for struct, the struct is treated as a single distinct event.
///
/// Deriving ::cqrs::Event for struct __requires__ specifying `#[event(type = "...")]` attribute (and only single
/// such attribute allowed per struct).
///
/// # Deriving for enums
///
/// When deriving ::cqrs::Event for enum, the enum is treated as a sum-type representing an event from a set of
/// possible events.
///
/// In practice this means, that ::cqrs::Event can only be derived for a enum when all variants of such enum
/// have exactly one field (variant can be both a tuple-variant or a struct-variant) and the field have to
/// implement ::cqrs::Event itself.
///
/// Generated implementation of `::cqrs::Event::event_type` would match on all variants and proxy calls to each
/// variant's field.
///
/// # Examples
/// ```
/// use cqrs_codegen::Event;
///
/// #[derive(Event)]
/// #[event(type = "user.created")]
/// struct UserCreated;
///
/// #[derive(Event)]
/// #[event(type = "user.removed")]
/// struct UserRemoved;
///
/// #[derive(Event)]
/// enum UserEvents {
///     UserCreated(UserCreated),
///     UserRemoved(UserRemoved),
/// }
/// ```
#[proc_macro_derive(Event, attributes(event))]
pub fn event(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    expand(input, event::event)
}

// ...

type MacroImpl = fn(syn::DeriveInput) -> syn::parse::Result<proc_macro2::TokenStream>;

fn expand(input: proc_macro::TokenStream, macro_impl: MacroImpl) -> proc_macro::TokenStream {
    let result = syn::parse(input).and_then(|input| macro_impl(input));
    match result {
        Ok(result) => result.into(),
        Err(error) => error.to_compile_error().into(),
    }
}
