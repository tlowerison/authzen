use convert_case::{Case, Casing};
use proc_macro2::TokenStream;
use syn::parse::{Error, Parse, ParseStream};
use syn::parse2;

const FIELD_ATTRIBUTE: &str = "id";

#[derive(Clone, Debug)]
struct EnumAttribute {
    uuid_literal: syn::LitStr,
}

impl Parse for EnumAttribute {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let content;
        parenthesized!(content in input);
        Ok(Self {
            uuid_literal: content.parse()?,
        })
    }
}

pub fn derive_enum(item: TokenStream) -> Result<TokenStream, Error> {
    let ast: syn::DeriveInput = parse2(item)?;

    let vis = &ast.vis;
    let ident = &ast.ident;
    let ident_str = format!("{ident}");
    let ident_snake = format_ident!("{}", ident.to_string().to_case(Case::Snake));
    let mod_name = format_ident!("diesel_util_enum_{}", ident.to_string().to_case(Case::Snake));

    let data_enum = match &ast.data {
        syn::Data::Enum(data_enum) => data_enum,
        _ => {
            return Err(Error::new_spanned(ast, "Enum can only be derived on enum types"));
        }
    };

    let num_variants = data_enum.variants.len();
    let mut static_id_idents: Vec<syn::Ident> = Vec::with_capacity(num_variants);
    let mut static_id_ident_strs: Vec<String> = Vec::with_capacity(num_variants);
    let mut static_id_lit_strs: Vec<syn::LitStr> = Vec::with_capacity(num_variants);
    let mut variant_idents: Vec<&syn::Ident> = Vec::with_capacity(num_variants);

    for variant in &data_enum.variants {
        if !matches!(variant.fields, syn::Fields::Unit) {
            return Err(Error::new_spanned(
                variant,
                "variant must be unit type aka have no fields",
            ));
        }

        let uuid_attrs = variant
            .attrs
            .iter()
            .filter(|attr| attr.path.is_ident(FIELD_ATTRIBUTE))
            .collect::<Vec<_>>();
        if uuid_attrs.len() != 1 {
            return Err(Error::new_spanned(
                variant,
                r#"variant must specify exactly one uuid literal using the #[uuid("00000000-0000-0000-0000-000000000000")] attribute"#,
            ));
        }

        static_id_idents.push(format_ident!(
            "{}_{}_ID",
            ident_str.to_case(Case::ScreamingSnake),
            variant.ident.to_string().to_case(Case::ScreamingSnake)
        ));
        static_id_ident_strs.push(format!("{ident}::{}", variant.ident));

        let lit_str: TokenStream = uuid_attrs[0].tokens.clone();
        static_id_lit_strs.push(parse2::<EnumAttribute>(lit_str)?.uuid_literal);
        variant_idents.push(&variant.ident);
    }

    let tokens = quote! {
        #vis use #mod_name::*;
        #vis mod #mod_name {
            use super::*;
            use diesel_util::diesel::backend::{Backend, RawValue};
            use diesel_util::diesel::deserialize::{self, FromSql};
            use diesel_util::diesel::serialize::{self, Output, ToSql};
            use diesel_util::diesel::sql_types;
            use diesel_util::Enum;
            use diesel_util::uuid as uuid;
            use std::borrow::Borrow;

            #(
               #vis static #static_id_idents: uuid::Uuid = uuid::uuid!(#static_id_lit_strs);
            )*

            impl Borrow<uuid::Uuid> for #ident {
                fn borrow(&self) -> &uuid::Uuid {
                    match self {
                        #(
                            #ident::#variant_idents => &#static_id_idents,
                        )*
                    }
                }
            }

            impl From<uuid::Uuid> for #ident {
                fn from(id: uuid::Uuid) -> Self {
                    #(
                        if id == #static_id_idents {
                            return #ident::#variant_idents;
                        }
                    )*
                    let err_msg_preamble = concat!("unexpected value for enum `", #ident_str, "`: ");
                    panic!("{}", err_msg_preamble.to_string() + &id.to_string())
                }
            }

            impl From<#ident> for uuid::Uuid {
                fn from(#ident_snake: #ident) -> Self {
                    match #ident_snake {
                        #(
                            #ident::#variant_idents => #static_id_idents,
                        )*
                    }
                }
            }

            impl<DB: Backend> ToSql<sql_types::Uuid, DB> for #ident
            where
                uuid::Uuid: ToSql<sql_types::Uuid, DB>,
            {
                fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, DB>) -> serialize::Result {
                    match *self {
                        #(
                            #ident::#variant_idents => #static_id_idents.to_sql(out),
                        )*
                    }
                }
            }

            impl<DB: Backend> FromSql<sql_types::Uuid, DB> for #ident
            where
                uuid::Uuid: FromSql<sql_types::Uuid, DB>,
            {
                fn from_sql(bytes: RawValue<'_, DB>) -> deserialize::Result<Self> {
                    let id = uuid::Uuid::from_sql(bytes)?;
                    #(
                        if id.as_bytes() == #static_id_idents.as_bytes() {
                            return Ok(#ident::#variant_idents);
                        }
                    )*
                    let err_msg_preamble = concat!("unexpected value for enum `", #ident_str, "`: ");
                    Err((err_msg_preamble.to_string() + &id.to_string()).into())
                }
            }

            impl From<&#ident> for uuid::Uuid {
                fn from(#ident_snake: &#ident) -> Self {
                    match #ident_snake {
                        #(
                            #ident::#variant_idents => #static_id_idents,
                        )*
                    }
                }
            }

            impl Enum for #ident {
                type Variants = std::array::IntoIter<Self, #num_variants>;
                fn id(&self) -> uuid::Uuid {
                    self.into()
                }
                fn variants() -> Self::Variants {
                    [#(Self::#variant_idents,)*].into_iter()
                }
                fn with_title(&self) -> diesel_util::WithTitle<Self> {
                    diesel_util::WithTitle {
                        id: self.into(),
                        title: self.clone(),
                    }
                }
            }
        }
    };

    Ok(tokens)
}

#[cfg(tests)]
mod test {
    use super::*;

    #[test]
    fn test_uuid_id() -> Result<(), Error> {
        derive_enum(quote!(
            pub enum Food {
                #[id = "94ba028b-b74b-4af4-8495-de7f468ffc76"]
                Carbs,
                #[id = "8cacc7ff-c52e-4193-ab2f-758089c1ccc6"]
                Ramen,
            }
        ))?;

        Ok(())
    }
}
