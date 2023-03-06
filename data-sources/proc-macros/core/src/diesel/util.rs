use proc_macro2::TokenTree;
use syn::parse::{Error, Parse, ParseStream};
use syn::{DeriveInput, Ident, Token};

#[derive(Default, Clone)]
pub struct DieselAttribute {
    pub table_name: Option<Ident>,
}

impl Parse for DieselAttribute {
    fn parse(parse_stream: ParseStream) -> syn::Result<Self> {
        let mut table_name: Option<Ident> = None;

        while !parse_stream.is_empty() {
            if let Some::<Ident>(arg) = parse_stream.parse().ok() {
                if &*format!("{arg}") == "table_name" {
                    let eq: Option<Token![=]> = parse_stream.parse().ok();
                    if eq.is_some() {
                        let ident: Option<Ident> = parse_stream.parse().ok();
                        if let Some(ident) = ident {
                            table_name = Some(ident);
                        } else {
                            let _: TokenTree = parse_stream.parse()?;
                        }
                    } else {
                        let _: TokenTree = parse_stream.parse()?;
                    }
                } else {
                    let _: TokenTree = parse_stream.parse()?;
                }
            } else {
                let _: TokenTree = parse_stream.parse()?;
            }
        }

        Ok(Self { table_name })
    }
}

impl TryFrom<&DeriveInput> for DieselAttribute {
    type Error = Error;
    fn try_from(ast: &DeriveInput) -> Result<Self, Self::Error> {
        for attr in ast.attrs.iter() {
            if attr.path.is_ident("diesel") {
                return attr.parse_args();
            }
        }
        Ok(Default::default())
    }
}

pub(crate) fn get_primary_key(ast: &syn::DeriveInput) -> syn::Ident {
    ast.attrs
        .iter()
        .find(|attr| {
            if let Some(attr_ident) = attr.path.get_ident() {
                if &*format!("{attr_ident}") == "primary_key" {
                    return true;
                }
            }
            false
        })
        .map(|attr| attr.parse_args().unwrap())
        .unwrap_or_else(|| format_ident!("id"))
}
