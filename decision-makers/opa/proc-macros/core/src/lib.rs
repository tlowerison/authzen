#![cfg_attr(all(doc, CHANNEL_NIGHTLY), feature(doc_auto_cfg))]

use authzen_proc_macro_util::{add_bounds_to_generics, find_field_attribute_in_struct, MatchedAttribute};
use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{parse2, parse_quote, punctuated::Punctuated, Error, Token};

#[derive(Clone, Debug)]
pub struct OPAContextAccountSessionAttributeArgs {
    pub id_ty: syn::Type,
    pub fields_ty: Option<syn::Type>,
}

pub fn opa_context_core(item: TokenStream) -> Result<TokenStream, Error> {
    let ast: syn::DeriveInput = parse2(item)?;
    let ident = &ast.ident;

    match &ast.data {
        syn::Data::Struct(_) => {}
        _ => {
            return Err(Error::new_spanned(
                ast,
                "OPAContext can only be derived on struct types",
            ))
        }
    };

    let MatchedAttribute {
        field_accessor: account_session_field_accessor,
        field: account_session_field,
        attr: account_session_attribute,
    } = find_field_attribute_in_struct("account_session", &ast)?;
    let MatchedAttribute {
        field_accessor: opa_client_field_accessor,
        field: opa_client_field,
        ..
    } = find_field_attribute_in_struct("opa_client", &ast)?;

    let account_session_field_type = &account_session_field.ty;
    let opa_client_field_type = &opa_client_field.ty;

    let OPAContextAccountSessionAttributeArgs {
        id_ty: account_id_ty,
        fields_ty: account_session_fields_ty,
    } = account_session_attribute.parse_args()?;
    let account_session_fields_ty: syn::Type = account_session_fields_ty.unwrap_or_else(|| parse_quote!(()));

    let (_, ty_generics, _) = ast.generics.split_for_impl();

    let mut opa_context_trait_generics = ast.generics.clone();

    add_bounds_to_generics(
        &mut opa_context_trait_generics,
        [parse_quote!(Send), parse_quote!(Sync)],
        None,
    );
    add_bounds_to_generics(
        &mut opa_context_trait_generics,
        [
            parse_quote!(authzen::decision_makers::opa::authzen_session::BorrowAccountSession<#account_id_ty, #account_session_fields_ty>),
        ],
        Some(&parse_quote!(#account_session_field_type)),
    );
    add_bounds_to_generics(
        &mut opa_context_trait_generics,
        [parse_quote!(Borrow<authzen::decision_makers::opa::OPAClient>)],
        Some(&parse_quote!(#opa_client_field_type)),
    );

    let (opa_context_impl_generics, _, opa_context_where_clause) = opa_context_trait_generics.split_for_impl();

    let tokens = quote! {
        #[authzen::decision_makers::opa::authzen_opa_async_trait]
        impl #opa_context_impl_generics authzen::decision_makers::opa::OPAContext for #ident #ty_generics #opa_context_where_clause {
            type AccountId = #account_id_ty;
            type AccountSessionFields = #account_session_fields_ty;
            fn account_session(&self) -> Option<&authzen::decision_makers::opa::authzen_session::AccountSession<Self::AccountId, Self::AccountSessionFields>> {
                self.#account_session_field_accessor.borrow_account_session()
            }
            fn opa_client(&self) -> &authzen::decision_makers::opa::OPAClient {
                self.#opa_client_field_accessor.borrow()
            }
        }
    };

    Ok(tokens)
}

pub fn opa_tx_cache_context_core(item: TokenStream) -> Result<TokenStream, Error> {
    let ast: syn::DeriveInput = parse2(item)?;
    let ident = &ast.ident;

    match &ast.data {
        syn::Data::Struct(_) => {}
        _ => {
            return Err(Error::new_spanned(
                ast,
                "OPATxCacheContext can only be derived on struct types",
            ))
        }
    };

    let MatchedAttribute {
        field_accessor: db_field_accessor,
        field: db_field,
        ..
    } = find_field_attribute_in_struct("db", &ast)?;
    let MatchedAttribute {
        field_accessor: opa_tx_cache_client_field_accessor,
        field: opa_tx_cache_client_field,
        ..
    } = find_field_attribute_in_struct("opa_tx_cache_client", &ast)?;

    let db_field_type = &db_field.ty;
    let opa_tx_cache_client_type = &opa_tx_cache_client_field.ty;

    let (_, ty_generics, _) = ast.generics.split_for_impl();

    let mut opa_tx_cache_context_trait_generics = ast.generics.clone();
    add_bounds_to_generics(
        &mut opa_tx_cache_context_trait_generics,
        [parse_quote!(Send), parse_quote!(Sync)],
        None,
    );
    add_bounds_to_generics(
        &mut opa_tx_cache_context_trait_generics,
        [parse_quote!(authzen_diesel::_Db)],
        Some(&parse_quote!(#db_field_type)),
    );
    add_bounds_to_generics(
        &mut opa_tx_cache_context_trait_generics,
        [
            parse_quote!(Clone),
            parse_quote!(authzen::decision_makers::opa::OPATxCacheClient),
        ],
        Some(&parse_quote!(#opa_tx_cache_client_type)),
    );

    let (opa_tx_cache_context_impl_generics, _, opa_tx_cache_context_where_clause) =
        opa_tx_cache_context_trait_generics.split_for_impl();

    let tokens = quote! {
        #[authzen::decision_makers::opa::authzen_opa_async_trait]
        impl #opa_tx_cache_context_impl_generics authzen::decision_makers::opa::OPATxCacheContext for #ident #ty_generics #opa_tx_cache_context_where_clause {
            type TxCacheClient = #opa_tx_cache_client_type;
            fn opa_tx_cache_client(&self) -> Self::TxCacheClient {
                self.#opa_tx_cache_client_field_accessor.clone()
            }
            fn transaction_id(&self) -> Option<Uuid> {
                self.#db_field_accessor.borrow().tx_id()
            }
        }
    };

    Ok(tokens)
}

pub fn opa_type_core(item: TokenStream) -> Result<TokenStream, Error> {
    let ast: syn::DeriveInput = parse2(item)?;
    let ident = &ast.ident;

    let data_struct = match &ast.data {
        syn::Data::Struct(data_struct) => data_struct,
        _ => return Err(Error::new_spanned(ast, "OPAType can only be derived on struct types")),
    };

    if data_struct.fields.len() != 1 {
        return Err(Error::new_spanned(
            ast,
            "OPAType can only be derived on tuple structs with one one field",
        ));
    }

    let field = match &data_struct.fields {
        syn::Fields::Unnamed(fields_unnamed) => fields_unnamed.unnamed.first().unwrap(),
        _ => return Err(Error::new_spanned(ast, "OPAType can only be derived on tuple structs")),
    };

    let (lifetime, inner_ty) = match &field.ty {
        syn::Type::Path(path_type) => match &path_type.path.segments.last().unwrap().arguments {
            syn::PathArguments::AngleBracketed(angle_bracketed_generic_arguments) => {
                if angle_bracketed_generic_arguments.args.len() != 2 {
                    return Err(Error::new_spanned(&field.ty, "OPAType field must have type OPAType<'lifetime, ...>: unexpected number of generic parameters provided to OPAType"));
                }
                let lifetime = match angle_bracketed_generic_arguments.args.first().unwrap() {
                    syn::GenericArgument::Lifetime(lifetime) => lifetime,
                    _ => return Err(Error::new_spanned(&field.ty, "OPAType field must have type OPAType<'lifetime, ...>: expected first parameter provided to OPAType to be a lifetime")),
                };
                let inner_ty = match angle_bracketed_generic_arguments.args.last().unwrap() {
                    syn::GenericArgument::Type(inner_ty) => inner_ty,
                    _ => return Err(Error::new_spanned(&field.ty, "OPAType field must have type OPAType<'lifetime, ...>: expected last parameter provided to OPAType to be a type")),
                };
                (lifetime, inner_ty)
            },
            _ => return Err(Error::new_spanned(&field.ty, "OPAType field must have type OPAType<'lifetime, ...>: expected OPAType to use angle brackets for its generics")),
        },
        _ => return Err(Error::new_spanned(&field.ty, "OPAType field must have type OPAType<'lifetime, ...>: expected field to be a path type")),
    };

    let mut de_generics = ast.generics.clone();

    let de_lifetime: syn::GenericParam = parse_quote!('de);
    de_generics.params.insert(0, de_lifetime.clone());

    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();
    let (de_impl_generics, _, de_where_clause) = de_generics.split_for_impl();

    let tokens = quote! {
        impl #impl_generics authzen::decision_makers::opa::authzen_opa_serde::ser::Serialize for #ident #ty_generics #where_clause {
            #[inline]
            fn serialize<S: authzen::decision_makers::opa::authzen_opa_serde::ser::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                use authzen::decision_makers::opa::authzen_opa_serde::ser::Serialize;
                self.0.serialize(serializer)
            }
        }

        impl #de_impl_generics authzen::decision_makers::opa::authzen_opa_serde::de::Deserialize<#de_lifetime> for #ident #ty_generics #de_where_clause {
            #[inline]
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: authzen::decision_makers::opa::authzen_opa_serde::de::Deserializer<'de>,
            {
                use authzen::decision_makers::opa::authzen_opa_serde::de::Deserialize;
                authzen::decision_makers::opa::OPAType::deserialize(deserializer).map(Self)
            }
        }

        impl #impl_generics From<&#lifetime #inner_ty> for #ident #ty_generics #where_clause {
            fn from(value: &#lifetime #inner_ty) -> Self {
                Self(OPAType::from(value))
            }
        }

        impl #impl_generics From<#inner_ty> for #ident #ty_generics #where_clause {
            fn from(value: #inner_ty) -> Self {
                Self(OPAType::from(value))
            }
        }
    };

    Ok(tokens)
}

impl Parse for OPAContextAccountSessionAttributeArgs {
    fn parse(input: ParseStream<'_>) -> Result<Self, Error> {
        let mut id_ty: Option<syn::Type> = None;
        let mut fields_ty: Option<syn::Type> = None;
        for arg in Punctuated::<OPAContextAccountSessionAttributeArg, Token![,]>::parse_terminated(input)? {
            match &*arg.ident.to_string() {
                "id" => {
                    id_ty = Some(arg.ty);
                }
                "fields" => {
                    fields_ty = Some(arg.ty);
                }
                _ => {
                    return Err(Error::new_spanned(
                        &arg.ident,
                        format!("unexpected argument `{}`, only accepts `id` and `fields`", arg.ident),
                    ))
                }
            }
        }
        Ok(Self {
            id_ty: id_ty
                .ok_or_else(|| Error::new(Span::call_site(), "`id` arg required for `account_session` attribute"))?,
            fields_ty,
        })
    }
}

#[derive(Clone, Debug)]
pub struct OPAContextAccountSessionAttributeArg {
    pub ident: syn::Ident,
    pub ty: syn::Type,
}

impl Parse for OPAContextAccountSessionAttributeArg {
    fn parse(input: ParseStream<'_>) -> Result<Self, Error> {
        let ident: syn::Ident = input.parse()?;
        let _eq: Token![=] = input.parse()?;
        let ty: syn::Type = input.parse()?;
        Ok(Self { ident, ty })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use authzen_proc_macro_util::pretty;

    #[test]
    fn test_opa_context_basic() -> Result<(), Error> {
        let received = opa_context_core(quote!(
            pub struct Context<A, D, O1, O2> {
                #[account_session(id = Uuid, fields = AccountSessionFields)]
                pub account_session: A,
                pub db: D,
                #[opa_client]
                pub opa_client: O1,
                pub opa_tx_cache_client: O2,
            }
        ))?;

        let expected = quote!(
            #[authzen::decision_makers::opa::authzen_opa_async_trait]
            impl<
                    A: Send
                        + Sync
                        + authzen::decision_makers::opa::authzen_session::BorrowAccountSession<Uuid, AccountSessionFields>,
                    D: Send + Sync,
                    O1: Send + Sync + Borrow<authzen::decision_makers::opa::OPAClient>,
                    O2: Send + Sync,
                > authzen::decision_makers::opa::OPAContext for Context<A, D, O1, O2>
            {
                type AccountId = Uuid;
                type AccountSessionFields = AccountSessionFields;
                fn account_session(
                    &self,
                ) -> Option<
                    &authzen::decision_makers::opa::authzen_session::AccountSession<
                        Self::AccountId,
                        Self::AccountSessionFields,
                    >,
                > {
                    self.account_session.borrow_account_session()
                }
                fn opa_client(&self) -> &authzen::decision_makers::opa::OPAClient {
                    self.opa_client.borrow()
                }
            }
        );

        assert_eq!(pretty(&expected)?, pretty(&received)?);

        Ok(())
    }

    #[test]
    fn test_opa_context_with_additional_bounds() -> Result<(), Error> {
        let received = opa_context_core(quote!(
            pub struct Context<A: Send + Sync, D: Foo, O1: Yo, O2: Test> {
                #[account_session(id = Uuid)]
                pub account_session: A,
                pub db: D,
                #[opa_client]
                pub opa_client: O1,
                pub opa_tx_cache_client: O2,
            }
        ))?;

        let expected = quote!(
            #[authzen::decision_makers::opa::authzen_opa_async_trait]
            impl<
                    A: Send + Sync + authzen::decision_makers::opa::authzen_session::BorrowAccountSession<Uuid, ()>,
                    D: Foo + Send + Sync,
                    O1: Yo + Send + Sync + Borrow<authzen::decision_makers::opa::OPAClient>,
                    O2: Test + Send + Sync,
                > authzen::decision_makers::opa::OPAContext for Context<A, D, O1, O2>
            {
                type AccountId = Uuid;
                type AccountSessionFields = ();
                fn account_session(
                    &self,
                ) -> Option<
                    &authzen::decision_makers::opa::authzen_session::AccountSession<
                        Self::AccountId,
                        Self::AccountSessionFields,
                    >,
                > {
                    self.account_session.borrow_account_session()
                }
                fn opa_client(&self) -> &authzen::decision_makers::opa::OPAClient {
                    self.opa_client.borrow()
                }
            }
        );

        assert_eq!(pretty(&expected)?, pretty(&received)?);

        Ok(())
    }

    #[test]
    fn test_opa_tx_cache_context_basic() -> Result<(), Error> {
        let received = opa_tx_cache_context_core(quote!(
            pub struct Context<A, D, O1, O2> {
                pub account_session: A,
                #[db]
                pub db: D,
                pub opa_client: O1,
                #[opa_tx_cache_client]
                pub opa_tx_cache_client: O2,
            }
        ))?;

        let expected = quote!(
            #[authzen::decision_makers::opa::authzen_opa_async_trait]
            impl<
                    A: Send + Sync,
                    D: Send + Sync + authzen_diesel::_Db,
                    O1: Send + Sync,
                    O2: Send + Sync + Clone + authzen::decision_makers::opa::OPATxCacheClient,
                > authzen::decision_makers::opa::OPATxCacheContext for Context<A, D, O1, O2>
            {
                type TxCacheClient = O2;
                fn opa_tx_cache_client(&self) -> Self::TxCacheClient {
                    self.opa_tx_cache_client.clone()
                }
                fn transaction_id(&self) -> Option<Uuid> {
                    self.db.borrow().tx_id()
                }
            }
        );

        assert_eq!(pretty(&expected)?, pretty(&received)?);

        Ok(())
    }

    #[test]
    fn test_opa_tx_cache_context_with_additional_bounds() -> Result<(), Error> {
        let received = opa_tx_cache_context_core(quote!(
            pub struct Context<A: Send + Sync, D: Foo, O1: Yo, O2: Test> {
                pub account_session: A,
                #[db]
                pub db: D,
                pub opa_client: O1,
                #[opa_tx_cache_client]
                pub opa_tx_cache_client: O2,
            }
        ))?;

        let expected = quote!(
            #[authzen::decision_makers::opa::authzen_opa_async_trait]
            impl<
                    A: Send + Sync,
                    D: Foo + Send + Sync + authzen_diesel::_Db,
                    O1: Yo + Send + Sync,
                    O2: Test + Send + Sync + Clone + authzen::decision_makers::opa::OPATxCacheClient,
                > authzen::decision_makers::opa::OPATxCacheContext for Context<A, D, O1, O2>
            {
                type TxCacheClient = O2;
                fn opa_tx_cache_client(&self) -> Self::TxCacheClient {
                    self.opa_tx_cache_client.clone()
                }
                fn transaction_id(&self) -> Option<Uuid> {
                    self.db.borrow().tx_id()
                }
            }
        );

        assert_eq!(pretty(&expected)?, pretty(&received)?);

        Ok(())
    }

    #[test]
    fn test_opa_context_missing_account_session_attribute() -> Result<(), Error> {
        let err = opa_context_core(quote!(
            pub struct Context<A, D, O1, O2> {
                pub account_session: A,
                pub db: D,
                #[opa_client]
                pub opa_client: O1,
                pub opa_tx_cache_client: O2,
            }
        ))
        .unwrap_err();

        assert_eq!(
            "exactly one field must be marked with the `#[account_session]` attribute",
            format!("{err}")
        );

        Ok(())
    }

    #[test]
    fn test_opa_context_missing_opa_client_attribute() -> Result<(), Error> {
        let err = opa_context_core(quote!(
            pub struct Context<A, D, O1, O2> {
                #[account_session]
                pub account_session: A,
                pub db: D,
                pub opa_client: O1,
                pub opa_tx_cache_client: O2,
            }
        ))
        .unwrap_err();

        assert_eq!(
            "exactly one field must be marked with the `#[opa_client]` attribute",
            format!("{err}")
        );

        Ok(())
    }

    #[test]
    fn test_opa_tx_cache_context_missing_db_attribute() -> Result<(), Error> {
        let err = opa_tx_cache_context_core(quote!(
            pub struct Context<A, D, O1, O2> {
                pub account_session: A,
                pub db: D,
                pub opa_client: O1,
                #[opa_tx_cache_client]
                pub opa_tx_cache_client: O2,
            }
        ))
        .unwrap_err();

        assert_eq!(
            "exactly one field must be marked with the `#[db]` attribute",
            format!("{err}")
        );

        Ok(())
    }

    #[test]
    fn test_opa_tx_cache_context_missing_opa_tx_cache_client_attribute() -> Result<(), Error> {
        let err = opa_tx_cache_context_core(quote!(
            pub struct Context<A, D, O1, O2> {
                pub account_session: A,
                #[db]
                pub db: D,
                pub opa_client: O1,
                pub opa_tx_cache_client: O2,
            }
        ))
        .unwrap_err();

        assert_eq!(
            "exactly one field must be marked with the `#[opa_tx_cache_client]` attribute",
            format!("{err}")
        );

        Ok(())
    }
}
