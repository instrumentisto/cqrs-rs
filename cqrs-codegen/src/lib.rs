mod event;
mod utility;

extern crate proc_macro;

use proc_macro2;
use syn;

// ...

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
