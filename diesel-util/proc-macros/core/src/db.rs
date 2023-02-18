use proc_macro2::TokenStream;
use proc_macro_util::add_bounds_to_generics;
use std::str::FromStr;
use syn::parse::Error;
use syn::parse2;

pub fn derive_db(item: TokenStream) -> Result<TokenStream, Error> {
    let ast: syn::DeriveInput = parse2(item)?;

    let ident = &ast.ident;

    let data_struct = match &ast.data {
        syn::Data::Struct(data_struct) => data_struct,
        _ => return Err(Error::new_spanned(ast, "Db can only be derived on struct types")),
    };

    let mut should_box_tx_fn = true;
    if let Some(attr) = ast.attrs.iter().find(|attr| attr.path.is_ident("db")) {
        let ident = attr.parse_args::<syn::Ident>()?;
        match &*ident.to_string() {
            "no_tx_fn_box" => should_box_tx_fn = false,
            _ => {}
        };
    }

    let mut db_field_and_index: Option<(usize, &syn::Field)> = None;
    for (index, db_field) in data_struct.fields.iter().enumerate() {
        for attr in &db_field.attrs {
            if attr.path.is_ident("db") {
                if db_field_and_index.is_some() {
                    return Err(Error::new_spanned(
                        attr,
                        "`#[db]` attribute cannot be used more than once",
                    ));
                } else {
                    db_field_and_index = Some((index, db_field))
                }
            }
        }
    }
    let (db_field_index, db_field) = match db_field_and_index {
        Some(db_field_and_index) => db_field_and_index,
        None => {
            return Err(Error::new_spanned(
                ast,
                "Db requires exactly one field to be marked with the `#[db]` attribute",
            ));
        }
    };

    let db_field_accessor = match db_field.ident.as_ref() {
        Some(ident) => quote!(#ident),
        None => TokenStream::from_str(&format!("{db_field_index}")).unwrap(),
    };
    let db_field_type = &db_field.ty;

    let (_, ty_generics, _) = ast.generics.split_for_impl();

    let with_tx_connection_lifetime = quote!('with_tx_connection);

    let mut db_trait_generics = ast.generics.clone();

    add_bounds_to_generics(
        &mut db_trait_generics,
        [
            parse_quote!(Clone),
            parse_quote!(std::fmt::Debug),
            parse_quote!(Send),
            parse_quote!(Sync),
        ],
        None,
    );

    if db_trait_generics.where_clause.is_none() {
        db_trait_generics.where_clause = Some(syn::WhereClause {
            where_token: Default::default(),
            predicates: Default::default(),
        });
    }
    db_trait_generics
        .where_clause
        .as_mut()
        .unwrap()
        .predicates
        .push(parse_quote!(#db_field_type: diesel_util::_Db));

    let (db_impl_generics, _, db_where_clause) = db_trait_generics.split_for_impl();

    let tx_connection_input = format_ident!("tx_connection");
    let tx_connection_constructor = match &data_struct.fields {
        syn::Fields::Named(_) => {
            let fields = data_struct
                .fields
                .iter()
                .enumerate()
                .map(|(i, field)| {
                    let field_ident = &field.ident;
                    if i == db_field_index {
                        quote!(#field_ident: #tx_connection_input)
                    } else {
                        quote!(#field_ident: self.#field_ident.clone())
                    }
                })
                .collect::<Vec<_>>();
            quote!(#ident { #(#fields,)* })
        }
        syn::Fields::Unnamed(_) => {
            let fields = data_struct
                .fields
                .iter()
                .enumerate()
                .map(|(i, _)| {
                    let field_ident = TokenStream::from_str(&format!("{i}")).unwrap();
                    if i == db_field_index {
                        quote!(#tx_connection_input)
                    } else {
                        quote!(self.#field_ident.clone())
                    }
                })
                .collect::<Vec<_>>();
            quote!(#ident(#(#fields,)*))
        }
        _ => unreachable!(),
    };

    if ast.generics.params.is_empty() {
        return Err(Error::new_spanned(ast, "Db requires at least one generic which must be used as the exact type value for the field marked with `#[db]`"));
    }

    let mut tx_connection_type_constructor = quote!(#ident<);
    for param in ast.generics.params.iter() {
        if let syn::GenericParam::Type(syn::TypeParam { ident, .. }) = param {
            let is_param_exact_db_field_type = if let syn::Type::Path(syn::TypePath { path, .. }) = db_field_type {
                if let Some(db_field_type_ident) = path.get_ident() {
                    ident == db_field_type_ident
                } else {
                    false
                }
            } else {
                false
            };
            if is_param_exact_db_field_type {
                tx_connection_type_constructor = quote!(#tx_connection_type_constructor <#ident as diesel_util::_Db>::TxConnection<#with_tx_connection_lifetime>,);
            } else {
                tx_connection_type_constructor = quote!(#tx_connection_type_constructor #ident,);
            }
        }
    }
    tx_connection_type_constructor = quote!(#tx_connection_type_constructor>);

    let sub_callback = format_ident!("sub_callback");
    let sub_callback = if should_box_tx_fn {
        quote!({
            let #sub_callback: Box<
                dyn for<'r> diesel_util::TxFn<
                    'a,
                    D::TxConnection<'r>,
                    diesel_util::scoped_futures::ScopedBoxFuture<'a, 'r, Result<T, E>>,
                >,
            > = Box::new(move |#tx_connection_input| {
                let tx_connection = #tx_connection_constructor;
                callback(tx_connection)
            });
            #sub_callback
        })
    } else {
        quote!(
            move |#tx_connection_input| {
                let tx_connection = #tx_connection_constructor;
                callback(tx_connection)
            }
        )
    };

    let tokens = quote! {
        #[diesel_util::diesel_util_async_trait]
        impl #db_impl_generics diesel_util::_Db for #ident #ty_generics #db_where_clause {
            type Backend = <#db_field_type as diesel_util::_Db>::Backend;
            type AsyncConnection = <#db_field_type as diesel_util::_Db>::AsyncConnection;
            type Connection<'r> = <#db_field_type as diesel_util::_Db>::Connection<'r> where Self: 'r;
            type TxConnection<#with_tx_connection_lifetime> = #tx_connection_type_constructor;

            async fn query<'a, F, T, E>(&self, f: F) -> Result<T, E>
            where
                F: for<'r> FnOnce(&'r mut Self::AsyncConnection) -> diesel_util::scoped_futures::ScopedBoxFuture<'a, 'r, Result<T, E>>
                    + Send
                    + 'a,
                E: std::fmt::Debug + From<diesel_util::diesel::result::Error> + Send + 'a,
                T: Send + 'a
            {
                self.#db_field_accessor.query(f).await
            }

            async fn with_tx_connection<'a, F, T, E>(&self, f: F) -> Result<T, E>
            where
                F: for<'r> FnOnce(&'r mut Self::AsyncConnection) -> diesel_util::scoped_futures::ScopedBoxFuture<'a, 'r, Result<T, E>>
                    + Send
                    + 'a,
                E: std::fmt::Debug + From<diesel_util::diesel::result::Error> + Send + 'a,
                T: Send + 'a
            {
                self.#db_field_accessor.with_tx_connection(f).await
            }

            fn tx_id(&self) -> Option<diesel_util::uuid::Uuid> {
                self.#db_field_accessor.tx_id()
            }

            async fn tx_cleanup<F, E>(&self, f: F)
            where
                F: for<'r> diesel_util::TxCleanupFn<'r, Self::AsyncConnection, E>,
                E: Into<diesel_util::TxCleanupError> + 'static
            {
                self.#db_field_accessor.tx_cleanup(f).await
            }

            async fn tx<'life0, 'a, T, E, F>(&'life0 self, callback: F) -> Result<T, E>
            where
                F: for<'r> diesel_util::TxFn<'a, Self::TxConnection<'r>, diesel_util::scoped_futures::ScopedBoxFuture<'a, 'r, Result<T, E>>>,
                E: std::fmt::Debug + From<diesel_util::diesel::result::Error> + From<diesel_util::TxCleanupError> + Send + 'a,
                T: Send + 'a,
                'life0: 'a
            {
                self.#db_field_accessor.tx(#sub_callback).await
            }
        }
    };

    Ok(tokens)
}
