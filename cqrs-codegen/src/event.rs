use proc_macro2;

use quote::quote;

use syn::{
    self,
    parse::{Error, Result},
    spanned::Spanned as _,
};

use synstructure;

use crate::utility;

// ...

pub fn event(input: syn::DeriveInput) -> Result<proc_macro2::TokenStream> {
    match input.data {
        syn::Data::Struct(_) => derive_struct(input),
        syn::Data::Enum(_) => derive_enum(input),
        syn::Data::Union(data) => Err(Error::new(data.union_token.span(), "Event can only be derived for structs and enums")),
    }
}


fn derive_struct(input: syn::DeriveInput) -> Result<proc_macro2::TokenStream> {
    let syn::DeriveInput {
        attrs,
        ident,
        generics,
        ..
    } = input;

    let meta = utility::get_nested_meta(attrs, "event")?
        .ok_or_else(|| Error::new(proc_macro2::Span::call_site(), "Expected to find #[event(...)] attribute"))?;

    let event_type = parse_event_type_from_nested_meta(meta)?;

    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let render = quote! {
        impl#impl_generics #ident#ty_generics #where_clause {
            pub const EVENT_TYPE: ::cqrs::EventType = #event_type;
        }

        impl#impl_generics ::cqrs::Event for #ident#ty_generics #where_clause {
            fn event_type(&self) -> ::cqrs::EventType {
                Self::EVENT_TYPE
            }
        }
    };

    Ok(render)
}

fn parse_event_type_from_nested_meta(meta: syn::punctuated::Punctuated<syn::NestedMeta, syn::Token![,]>) -> Result<String> {
    const WRONG_FORMAT: &str = "Wrong format; proper format is #[event(type = \"...\")]";

    let mut event_type = None;

    meta.into_iter().try_for_each(|meta| -> Result<_> {
        match meta {
            syn::NestedMeta::Meta(meta) => {
                match meta {
                    syn::Meta::NameValue(meta) => {
                        if meta.path.is_ident("type") {
                            match meta.lit {
                                syn::Lit::Str(lit) => {
                                    update_if_none_or(&mut event_type, lit.value(), || Error::new(lit.span(), "Too many #[event(type = \"...\")] attributes specified; single attribute allowed"))
                                },
                                _ => Err(Error::new(meta.lit.span(), WRONG_FORMAT)),
                            }
                        } else {
                            Err(Error::new(meta.span(), WRONG_FORMAT))
                        }
                    },
                    _ => Err(Error::new(meta.span(), WRONG_FORMAT)),
                }
            },
            _ => Err(Error::new(meta.span(), WRONG_FORMAT)),
        }
    })?;

    event_type.ok_or_else(|| Error::new(proc_macro2::Span::call_site(), "Expected to find #[event(type = \"...\")] attribute"))
}

fn update_if_none_or<T, E>(option: &mut Option<T>, value: T, error: E) -> Result<()>
    where E: FnOnce() -> Error
{
    if option.is_none() {
        option.replace(value);
        Ok(())
    } else {
        Err(error())
    }
}


fn derive_enum(input: syn::DeriveInput) -> Result<proc_macro2::TokenStream> {
    let structure = synstructure::Structure::try_new(&input)?;
    derive_enum_impl(structure)
}

fn derive_enum_impl(mut structure: synstructure::Structure) -> Result<proc_macro2::TokenStream> {
    structure.variants().iter().try_for_each(|variant| -> Result<()> {
        let ast = variant.ast();
        if ast.fields.len() == 1 {
            Ok(())
        } else {
            Err(Error::new(ast.ident.span(), "Event can only be derived for enums with variants that have exactly one field"))
        }
    })?;

    structure.add_bounds(synstructure::AddBounds::Fields);

    structure.binding_name(|field, _| field.ident.as_ref().map_or_else(
        || syn::Ident::new("event", proc_macro2::Span::call_site()),
        |ident| ident.clone()
    ));

    let body = structure.each(|binding_info| {
        let ident = &binding_info.binding;
        quote!(#ident.event_type())
    });

    let render = structure.gen_impl(quote! {
        gen impl ::cqrs::Event for @Self {
            fn event_type(&self) -> ::cqrs::EventType {
                match *self {
                    #body
                }
            }
        }
    });

    Ok(render)
}


#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn event_struct() {
        let input = syn::parse_quote! {
            #[event(type = "event")]
            struct Event;
        };

        let output = derive_struct(input).unwrap();

        let expected_output = quote! {
            impl Event {
                pub const EVENT_TYPE: ::cqrs::EventType = "event";
            }

            impl ::cqrs::Event for Event {
                fn event_type(&self) -> ::cqrs::EventType {
                    Self::EVENT_TYPE
                }
            }
        };

        assert_eq!(output.to_string(), expected_output.to_string());
    }
}
