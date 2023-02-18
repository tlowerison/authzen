use crate::util::{get_primary_key, DieselAttribute};
use proc_macro2::TokenStream;
use syn::parse::{Error, Parse, ParseStream};

#[derive(Default)]
struct SoftDeleteAttribute {
    db_entity: Option<syn::Path>,
    deleted_at: Option<syn::Ident>,
}

impl Parse for SoftDeleteAttribute {
    fn parse(parse_stream: ParseStream) -> syn::Result<Self> {
        let mut db_entity: Option<syn::Path> = None;
        let mut deleted_at: Option<syn::Ident> = None;

        let mut first = true;
        while !parse_stream.is_empty() {
            if !first {
                let _comma: Token![,] = parse_stream.parse()?;
                if parse_stream.is_empty() {
                    break;
                }
            }
            let arg: syn::Ident = parse_stream.parse()?;
            let _eq: Token![=] = parse_stream.parse()?;

            match &*format!("{arg}") {
                "db_entity" => db_entity = Some(parse_stream.parse()?),
                "deleted_at" => deleted_at = Some(parse_stream.parse()?),
                _ => {
                    return Err(syn::Error::new_spanned(
                        arg,
                        "unrecorgnized argument to soft_delete attribute",
                    ))
                }
            };
            first = false;
        }

        Ok(SoftDeleteAttribute { db_entity, deleted_at })
    }
}

impl TryFrom<&syn::DeriveInput> for SoftDeleteAttribute {
    type Error = syn::parse::Error;
    fn try_from(ast: &syn::DeriveInput) -> Result<Self, Self::Error> {
        for attr in ast.attrs.iter() {
            if attr.path.is_ident("soft_delete") {
                return attr.parse_args();
            }
        }
        Ok(Default::default())
    }
}

pub fn derive_soft_delete(tokens: TokenStream) -> Result<TokenStream, Error> {
    let ast: syn::DeriveInput = syn::parse2(tokens)?;

    let diesel_attribute = DieselAttribute::try_from(&ast).expect("SoftDelete could not parse `diesel` attribute");
    let soft_delete_attribute =
        SoftDeleteAttribute::try_from(&ast).expect("SoftDelete could not parse `soft_delete` attribute");

    let ident = &ast.ident;
    let table_name = diesel_attribute
        .table_name
        .expect("SoftDelete was unable to extract table_name from a diesel(table_name = `...`) attribute");
    let deleted_at_column_name = soft_delete_attribute
        .deleted_at
        .unwrap_or_else(|| format_ident!("deleted_at"));

    let db_entity_path = soft_delete_attribute
        .db_entity
        .clone()
        .unwrap_or_else(|| parse_quote!(#ident));
    let soft_delete_ident = format_ident!("{}Delete", db_entity_path.segments.last().unwrap().ident);

    let data_struct = match &ast.data {
        syn::Data::Struct(data_struct) => data_struct,
        _ => {
            return Err(Error::new_spanned(
                ast,
                "SoftDelete can only be derived on struct types",
            ))
        }
    };
    let primary_key = get_primary_key(&ast);

    let primary_key_field = match data_struct
        .fields
        .iter()
        .find(|field| field.ident.as_ref() == Some(&primary_key))
    {
        Some(x) => x,
        _ => return Err(Error::new_spanned(ast, "SoftDelete could not locate primary_key field")),
    };

    let primary_key_ty = &primary_key_field.ty;

    let additional_impls = if soft_delete_attribute.db_entity.is_some() {
        let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();
        quote!(
            impl diesel_util::SoftDeletable for #db_entity_path {
                type DeletedAt = #table_name::#deleted_at_column_name;
            }

            impl #impl_generics diesel_util::DbDelete for #ident #ty_generics #where_clause {
                type DeletedAt = #table_name::#deleted_at_column_name;
                type DeletePatch<'a> = #soft_delete_ident;
            }
        )
    } else {
        quote!()
    };

    let tokens = quote! {
        impl diesel_util::SoftDeletable for #ident {
            type DeletedAt = #table_name::#deleted_at_column_name;
        }

        #[derive(AsChangeset, Clone, Debug, Identifiable, IncludesChanges)]
        #[diesel(table_name = #table_name)]
        pub struct #soft_delete_ident {
            id: #primary_key_ty,
            deleted_at: NaiveDateTime,
        }

        impl<T: std::borrow::Borrow<#primary_key_ty> + Clone> From<T> for #soft_delete_ident {
            fn from(id: T) -> Self {
                use std::borrow::Borrow;
                Self {
                    id: id.borrow().clone(),
                    #deleted_at_column_name: diesel_util::chrono::Utc::now().naive_utc(),
                }
            }
        }

        impl diesel_util::DbDelete for #db_entity_path {
            type DeletedAt = #table_name::#deleted_at_column_name;
            type DeletePatch<'a> = #soft_delete_ident;
        }

        #additional_impls
    };

    Ok(tokens)
}
