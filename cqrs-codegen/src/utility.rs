use syn::{
    self,
    parse::{Error, Result},
    spanned::Spanned as _,
};

// ...

pub fn get_nested_meta(attrs: Vec<syn::Attribute>, attr_name: &str) -> Result<Option<syn::punctuated::Punctuated<syn::NestedMeta, syn::Token![,]>>> {
    let mut iter = attrs.into_iter().filter_map(|attr| {
        attr.parse_meta().ok().and_then(|meta| {
            match meta {
                syn::Meta::List(meta) if meta.path.is_ident(attr_name) => Some(meta),
                _ => None,
            }
        })
    });

    let meta = iter.next();

    {
        let meta = iter.next();
        if meta.is_some() {
            return Err(Error::new(meta.span(), format!("Too many #[{}(...)] attributes specified; single attribute allowed", attr_name)));
        }
    }

    Ok(meta.map(|meta| meta.nested))
}
