use ::authzen_proc_macro_util::add_bounds_to_generics;
use ::proc_macro2::TokenStream;
use ::std::str::FromStr;
use ::syn::parse::Error;
use ::syn::parse2;

pub fn derive_transactional_data_source(item: TokenStream) -> Result<TokenStream, Error> {
    let ast: syn::DeriveInput = parse2(item)?;

    let ident = &ast.ident;

    let data_struct = match &ast.data {
        syn::Data::Struct(data_struct) => data_struct,
        _ => {
            return Err(Error::new_spanned(
                ast,
                "TransactionalDataSource can only be derived on struct types",
            ))
        }
    };

    let mut should_box_tx_fn = true;
    if let Some(attr) = ast.attrs.iter().find(|attr| attr.path.is_ident("data_source")) {
        let ident = attr.parse_args::<syn::Ident>()?;
        if ident == "no_tx_fn_box" {
            should_box_tx_fn = false;
        }
    }

    let mut data_source_field_and_index: Option<(usize, &syn::Field)> = None;
    for (index, data_source_field) in data_struct.fields.iter().enumerate() {
        for attr in &data_source_field.attrs {
            if attr.path.is_ident("data_source") {
                if data_source_field_and_index.is_some() {
                    return Err(Error::new_spanned(
                        attr,
                        "`#[data_source]` attribute cannot be used more than once",
                    ));
                } else {
                    data_source_field_and_index = Some((index, data_source_field))
                }
            }
        }
    }
    let (data_source_field_index, data_source_field) = match data_source_field_and_index {
        Some(data_source_field_and_index) => data_source_field_and_index,
        None => {
            return Err(Error::new_spanned(
                ast,
                "TransactionalDataSource requires exactly one field to be marked with the `#[data_source]` attribute",
            ));
        }
    };

    let data_source_field_accessor = match data_source_field.ident.as_ref() {
        Some(ident) => quote!(#ident),
        None => TokenStream::from_str(&format!("{data_source_field_index}")).unwrap(),
    };
    let data_source_field_type = &data_source_field.ty;

    let (_, ty_generics, _) = ast.generics.split_for_impl();

    let with_tx_connection_lifetime = quote!('with_tx_connection);

    let mut data_source_trait_generics = ast.generics.clone();

    add_bounds_to_generics(
        &mut data_source_trait_generics,
        [
            parse_quote!(Clone),
            parse_quote!(std::fmt::Debug),
            parse_quote!(Send),
            parse_quote!(Sync),
        ],
        None,
    );

    if data_source_trait_generics.where_clause.is_none() {
        data_source_trait_generics.where_clause = Some(syn::WhereClause {
            where_token: Default::default(),
            predicates: Default::default(),
        });
    }

    let mut transactional_data_source_trait_generics = data_source_trait_generics.clone();

    data_source_trait_generics
        .where_clause
        .as_mut()
        .unwrap()
        .predicates
        .push(parse_quote!(#data_source_field_type: ::authzen::data_sources::DataSource));

    transactional_data_source_trait_generics
        .where_clause
        .as_mut()
        .unwrap()
        .predicates
        .push(parse_quote!(#data_source_field_type: ::authzen::data_sources::TransactionalDataSource));

    let (data_source_impl_generics, _, data_source_where_clause) = data_source_trait_generics.split_for_impl();
    let (transactional_data_source_impl_generics, _, transactional_data_source_where_clause) =
        transactional_data_source_trait_generics.split_for_impl();

    let tx_connection_input = format_ident!("tx_connection");
    let tx_connection_constructor = match &data_struct.fields {
        syn::Fields::Named(_) => {
            let fields = data_struct
                .fields
                .iter()
                .enumerate()
                .map(|(i, field)| {
                    let field_ident = &field.ident;
                    if i == data_source_field_index {
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
                    if i == data_source_field_index {
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
        return Err(Error::new_spanned(ast, "TransactionalDataSource requires at least one generic which must be used as the exact type value for the field marked with `#[data_source]`"));
    }

    let mut tx_connection_type_constructor = quote!(#ident<);
    for param in ast.generics.params.iter() {
        if let syn::GenericParam::Type(syn::TypeParam { ident, .. }) = param {
            let is_param_exact_data_source_field_type =
                if let syn::Type::Path(syn::TypePath { path, .. }) = data_source_field_type {
                    if let Some(data_source_field_type_ident) = path.get_ident() {
                        ident == data_source_field_type_ident
                    } else {
                        false
                    }
                } else {
                    false
                };
            if is_param_exact_data_source_field_type {
                tx_connection_type_constructor = quote!(#tx_connection_type_constructor <#ident as ::authzen::data_sources::TransactionalDataSource>::TxConnection<#with_tx_connection_lifetime>,);
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
                dyn for<'r> ::authzen::data_sources::TxFn<
                    'a,
                    D::TxConnection<'r>,
                    ::authzen::data_sources::proc_macros_core::reexports::scoped_futures::ScopedBoxFuture<'a, 'r, Result<T, E>>,
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
        impl #data_source_impl_generics ::authzen::data_sources::DataSource for #ident #ty_generics #data_source_where_clause {
            type Backend = <#data_source_field_type as ::authzen::data_sources::DataSource>::Backend;
            type Error = <#data_source_field_type as ::authzen::data_sources::DataSource>::Error;
            type TransactionId = <#data_source_field_type as ::authzen::data_sources::DataSource>::TransactionId;

            fn transaction_id(&self) -> Option<Self::TransactionId> {
                self.#data_source_field_accessor.transaction_id()
            }
        }

        #[::authzen::data_sources::proc_macros_core::reexports::async_trait::async_trait]
        impl #transactional_data_source_impl_generics ::authzen::data_sources::TransactionalDataSource for #ident #ty_generics #transactional_data_source_where_clause {
            type AsyncConnection = <#data_source_field_type as ::authzen::data_sources::TransactionalDataSource>::AsyncConnection;
            type Connection<'r> = <#data_source_field_type as ::authzen::data_sources::TransactionalDataSource>::Connection<'r> where Self: 'r;
            type TxConnection<#with_tx_connection_lifetime> = #tx_connection_type_constructor;

            async fn query<'a, F, T, E>(&self, f: F) -> Result<T, E>
            where
                F: for<'r> FnOnce(&'r mut Self::AsyncConnection) -> ::authzen::data_sources::proc_macros_core::reexports::scoped_futures::ScopedBoxFuture<'a, 'r, Result<T, E>>
                    + Send
                    + 'a,
                E: ::std::fmt::Debug + From<Self::Error> + Send + 'a,
                T: Send + 'a
            {
                self.#data_source_field_accessor.query(f).await
            }

            async fn with_tx_connection<'a, F, T, E>(&self, f: F) -> Result<T, E>
            where
                F: for<'r> FnOnce(&'r mut Self::AsyncConnection) -> ::authzen::data_sources::proc_macros_core::reexports::scoped_futures::ScopedBoxFuture<'a, 'r, Result<T, E>>
                    + Send
                    + 'a,
                E: ::std::fmt::Debug + From<Self::Error> + Send + 'a,
                T: Send + 'a
            {
                self.#data_source_field_accessor.with_tx_connection(f).await
            }

            async fn tx_cleanup<F, E>(&self, f: F)
            where
                F: for<'r> ::authzen::data_sources::TxCleanupFn<'r, Self::AsyncConnection, E, Self::TransactionId>,
                E: Into<::authzen::data_sources::TxCleanupError> + 'static
            {
                self.#data_source_field_accessor.tx_cleanup(f).await
            }

            async fn tx<'life0, 'a, T, E, F>(&'life0 self, callback: F) -> Result<T, E>
            where
                F: for<'r> ::authzen::data_sources::TxFn<'a, Self::TxConnection<'r>, ::authzen::data_sources::proc_macros_core::reexports::scoped_futures::ScopedBoxFuture<'a, 'r, Result<T, E>>>,
                E: ::std::fmt::Debug + From<Self::Error> + From<::authzen::data_sources::TxCleanupError> + Send + 'a,
                T: Send + 'a,
                'life0: 'a
            {
                self.#data_source_field_accessor.tx(#sub_callback).await
            }
        }
    };

    Ok(tokens)
}
