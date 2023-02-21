#![cfg_attr(all(doc, CHANNEL_NIGHTLY), feature(doc_auto_cfg))]

#[macro_use]
extern crate derivative;
#[macro_use]
extern crate derive_more;
extern crate proc_macro;

mod match_path;

pub use match_path::*;

pub(crate) use proc_macro::TokenStream;
pub(crate) use proc_macro2::TokenStream as TokenStream2;
pub(crate) use quote::quote;
pub(crate) use std::fmt::Debug;
pub(crate) use std::str::FromStr;
pub(crate) use syn::{punctuated::Punctuated, Error};

#[macro_export]
macro_rules! ok_or_return_compile_error {
    ($expr:expr) => {
        match $expr {
            Ok(expr) => expr,
            Err(err) => return err.into_compile_error().into(),
        }
    };
}

/// Represents an attribute found on a struct which was matched
/// for by its path. Includes additional context about where the
/// attribute is found within the struct.
#[derive(Clone, Debug)]
pub struct MatchedAttribute<'a> {
    /// The entire attribute which was matched for based on its path.
    pub attr: &'a syn::Attribute,
    /// The field on which the matched attribute is found.
    pub field: &'a syn::Field,
    /// The token stream representation of the accessor to be used in dot notation for accessing this
    /// matched field in its parent struct. Has type [`proc_macro2::TokenStream`] instead of
    /// [`syn::Type`] in order to handle the case of unnamed fields, where the tokens are just an
    /// index.
    pub field_accessor: TokenStream2,
}

/// Finds a field attribute in a [`syn::DeriveInput`] matching the provided path.
/// Will throw error if the provided AST is not a struct.
/// Expects there to be exactly one matching attribute in the struct
pub fn find_field_attribute_in_struct<'a>(
    attr_path: &str,
    derive_input: &'a syn::DeriveInput,
) -> Result<MatchedAttribute<'a>, Error> {
    let data_struct = match &derive_input.data {
        syn::Data::Struct(data_struct) => data_struct,
        _ => return Err(Error::new_spanned(derive_input, "expected a struct data type")),
    };
    let mut field_and_index: Option<(usize, &syn::Field, &syn::Attribute)> = None;
    for (index, field) in data_struct.fields.iter().enumerate() {
        for attr in &field.attrs {
            if attr.path.is_ident(attr_path) {
                if field_and_index.is_some() {
                    return Err(Error::new_spanned(
                        attr,
                        format!("`#[{attr_path}]` attribute cannot be used more than once"),
                    ));
                } else {
                    field_and_index = Some((index, field, attr))
                }
            }
        }
    }
    let (field_index, field, attr) = match field_and_index {
        Some(field_and_index) => field_and_index,
        None => {
            return Err(Error::new_spanned(
                derive_input,
                format!("exactly one field must be marked with the `#[{attr_path}]` attribute"),
            ))
        }
    };

    let field_accessor = match field.ident.as_ref() {
        Some(ident) => quote!(#ident),
        None => TokenStream::from_str(&format!("{field_index}")).unwrap().into(),
    };

    Ok(MatchedAttribute {
        attr,
        field,
        field_accessor,
    })
}

/// Finds all field attributes in a [`syn::DeriveInput`] matching the provided path.
/// Will throw error if the provided AST is not a struct.
pub fn find_field_attributes_in_struct<'a>(
    attr_path: &str,
    derive_input: &'a syn::DeriveInput,
) -> Result<Vec<MatchedAttribute<'a>>, Error> {
    let data_struct = match &derive_input.data {
        syn::Data::Struct(data_struct) => data_struct,
        _ => return Err(Error::new_spanned(derive_input, "expected a struct data type")),
    };
    let mut fields_and_indices = Vec::<(usize, &syn::Field, &syn::Attribute)>::default();
    for (index, field) in data_struct.fields.iter().enumerate() {
        for attr in &field.attrs {
            if attr.path.is_ident(attr_path) {
                fields_and_indices.push((index, field, attr));
            }
        }
    }
    Ok(fields_and_indices
        .into_iter()
        .map(|(field_index, field, attr)| {
            let field_accessor = match field.ident.as_ref() {
                Some(ident) => quote!(#ident),
                None => TokenStream::from_str(&format!("{field_index}")).unwrap().into(),
            };
            MatchedAttribute {
                attr,
                field,
                field_accessor,
            }
        })
        .collect())
}

pub fn is_param_exact_field_type(param: &syn::GenericParam, field: &syn::Field) -> bool {
    let generic_param_type_ident = if let syn::GenericParam::Type(syn::TypeParam { ident, .. }) = param {
        ident
    } else {
        return false;
    };
    if let syn::Type::Path(syn::TypePath { path, .. }) = &field.ty {
        if let Some(field_type_ident) = path.get_ident() {
            generic_param_type_ident == field_type_ident
        } else {
            false
        }
    } else {
        false
    }
}

/// Adds all trait bounds to all generic params.
/// Skips adding bound to param if it already exists.
/// A bound is considered to already exist if:
/// - for lifetimes, exact equality between bounds is found
/// - for types, either the existing bound or the bound to add is a matched suffix of the other
///
/// # Example
/// ```
/// use authzen_proc_macro_util::add_bounds_to_generics;
/// use quote::quote;
/// use syn::parse2;
///
/// // check that a matched path will of a bound on a param will prevent it from being added to param
/// let mut generics = parse2(quote!(<A: Send, B: std::fmt::Debug>)).unwrap();
/// let bounds_to_add = [parse2(quote!(Debug)).unwrap()];
///
/// add_bounds_to_generics(&mut generics, bounds_to_add, None);
///
/// assert_eq!(parse2::<syn::Generics>(quote!(<A: Send + Debug, B: std::fmt::Debug>)).unwrap(), generics);
///
/// // check that a matched path will of a bound on a param will prevent it from being added to param
/// let mut generics = parse2(quote!(<A: Send, B: Debug>)).unwrap();
/// let bounds_to_add = [parse2(quote!(std::fmt::Debug)).unwrap()];
///
/// add_bounds_to_generics(&mut generics, bounds_to_add, None);
///
/// assert_eq!(parse2::<syn::Generics>(quote!(<A: Send + std::fmt::Debug, B: Debug>)).unwrap(), generics);
///
/// // check that a bound will be added to all params when none of them have it already
/// let mut generics = parse2(quote!(<A: Send, B: Debug>)).unwrap();
/// let bounds_to_add = [parse2(quote!(Sync)).unwrap()];
///
/// add_bounds_to_generics(&mut generics, bounds_to_add, None);
///
/// assert_eq!(parse2::<syn::Generics>(quote!(<A: Send + Sync, B: Debug + Sync>)).unwrap(), generics);
///
/// // check that specific idents can have bounds added by providing their ident
/// let mut generics = parse2(quote!(<A: Send, B: Debug>)).unwrap();
/// let bounds_to_add = [parse2(quote!(Sync)).unwrap()];
/// let ident: syn::Ident = parse2(quote!(A)).unwrap();
///
/// add_bounds_to_generics(&mut generics, bounds_to_add, Some(&ident));
///
/// assert_eq!(parse2::<syn::Generics>(quote!(<A: Send + Sync, B: Debug>)).unwrap(), generics);
/// ```
pub fn add_bounds_to_generics(
    generics: &mut syn::Generics,
    bounds_to_add: impl IntoIterator<Item = syn::TypeParamBound> + Debug,
    generic_param_ident: Option<&syn::Ident>,
) {
    let bounds_to_add = bounds_to_add.into_iter().collect::<Vec<_>>();
    for param in generics.params.iter_mut() {
        if let syn::GenericParam::Type(syn::TypeParam { bounds, ident, .. }) = param {
            if let Some(generic_param_ident) = generic_param_ident.as_ref() {
                let generic_param_ident: &syn::Ident = generic_param_ident;
                if *ident != *generic_param_ident {
                    continue;
                }
            }
            bounds_union(bounds, bounds_to_add.clone());
        }
    }
}

/// add bounds to an existing set of bounds while not adding any duplicates
pub fn bounds_union(
    bounds: &mut Punctuated<syn::TypeParamBound, syn::token::Add>,
    bounds_to_add: impl IntoIterator<Item = syn::TypeParamBound> + Debug,
) {
    for bound_to_add in bounds_to_add {
        let mut bound_to_add_exists = false;

        for bound in &*bounds {
            match (&bound_to_add, bound) {
                (
                    syn::TypeParamBound::Trait(syn::TraitBound { path: path_to_add, .. }),
                    syn::TypeParamBound::Trait(syn::TraitBound { path, .. }),
                ) => {
                    if match_path(path_to_add, path).is_ok() || match_path(path, path_to_add).is_ok() {
                        bound_to_add_exists = true;
                    }
                }
                (
                    syn::TypeParamBound::Lifetime(lifetime_bound_to_add),
                    syn::TypeParamBound::Lifetime(lifetime_bound),
                ) => {
                    if lifetime_bound_to_add == lifetime_bound {
                        bound_to_add_exists = true;
                    }
                }
                _ => {}
            }
        }

        if !bound_to_add_exists {
            bounds.push(bound_to_add.clone());
        }
    }
}

pub fn pretty(tokens: &TokenStream2) -> Result<String, Error> {
    let syntax_tree = syn::parse_file(&format!("{tokens}"))?;
    Ok(prettyplease::unparse(&syntax_tree))
}
