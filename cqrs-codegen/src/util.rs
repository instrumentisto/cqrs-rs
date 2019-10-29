//! Common crate utils used for codegen.

use syn::{
    parse::{Error, Result},
    punctuated::Punctuated,
    spanned::Spanned as _,
};

/// Finds attribute named with a given `name` and returns its inner parameters.
///
/// Errors if multiple attributes with the same `name` exist.
pub(crate) fn get_nested_meta(
    attrs: &[syn::Attribute],
    name: &str,
) -> Result<Option<Punctuated<syn::NestedMeta, syn::Token![,]>>> {
    let mut nested = None;
    for attr in attrs {
        if let syn::Meta::List(meta) = attr.parse_meta()? {
            if meta.path.is_ident(name) {
                if nested.is_some() {
                    return Err(Error::new(
                        meta.span(),
                        format!(
                            "Too many #[{}(...)] attributes specified, \
                             only single attribute is allowed",
                            name
                        ),
                    ));
                }
                nested.replace(meta.nested);
            }
        }
    }
    Ok(nested)
}
