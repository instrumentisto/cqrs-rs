mod event;
mod util;

#[cfg(not(feature = "watt"))]
macro_rules! export {
    ($fn:ident) => {
        pub use event::$fn;
    };
}

#[cfg(feature = "watt")]
macro_rules! export {
    ($fn:ident) => {
        #[no_mangle]
        pub extern "C" fn $fn(input: proc_macro2::TokenStream) -> proc_macro2::TokenStream {
            expand(syn::parse2(input), event::$fn)
        }
    }
}

pub fn expand<TS>(
    input: syn::Result<syn::DeriveInput>,
    macro_impl: fn(syn::DeriveInput) -> syn::Result<proc_macro2::TokenStream>,
) -> TS
where TS: From<proc_macro2::TokenStream>
{
    match input.and_then(|input| macro_impl(input)) {
        Ok(res) => res.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

export! {event_derive}
export! {registered_event_derive}
export! {versioned_event_derive}
