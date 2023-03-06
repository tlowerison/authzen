use authzen_proc_macro_util::add_bounds_to_generics;
use proc_macro2::TokenStream;
use syn::parse::Error;
use syn::parse2;

pub fn derive_db(item: TokenStream) -> Result<TokenStream, Error> {
    let ast: syn::DeriveInput = parse2(item)?;

    let ident = &ast.ident;

    let data_struct = match &ast.data {
        syn::Data::Struct(data_struct) => data_struct,
        _ => return Err(Error::new_spanned(ast, "Db can only be derived on struct types")),
    };

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
    let (_, data_source_field) = match data_source_field_and_index {
        Some(data_source_field_and_index) => data_source_field_and_index,
        None => {
            return Err(Error::new_spanned(
                ast,
                "Db requires exactly one field to be marked with the `#[data_source]` attribute",
            ));
        }
    };

    let data_source_field_type = &data_source_field.ty;

    let (_, ty_generics, _) = ast.generics.split_for_impl();

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
    let data_source_trait_predicates = &mut data_source_trait_generics.where_clause.as_mut().unwrap().predicates;
    data_source_trait_predicates
        .push(parse_quote!(#data_source_field_type: ::authzen::data_sources::diesel::connection::Db));
    data_source_trait_predicates.push(parse_quote!(
        <#data_source_field_type as ::authzen::data_sources::DataSource>::Backend:
            ::authzen::data_sources::proc_macros_core::diesel::reexports::diesel::backend::Backend
    ));
    data_source_trait_predicates.push(parse_quote!(
        <#data_source_field_type as ::authzen::data_sources::DataSource>::AsyncConnection:
            ::authzen::data_sources::proc_macros_core::diesel::reexports::diesel_async::AsyncConnection<Backend = <#data_source_field_type as ::authzen::data_sources::DataSource>::Backend>
    ));

    let (data_source_impl_generics, _, data_source_where_clause) = data_source_trait_generics.split_for_impl();

    if ast.generics.params.is_empty() {
        return Err(Error::new_spanned(ast, "Db requires at least one generic which must be used as the exact type value for the field marked with `#[db]`"));
    }

    let tokens = quote! {
        #[::authzen::data_sources::proc_macros_core::diesel::reexports::async_trait::async_trait]
        impl #data_source_impl_generics ::authzen::data_sources::DataSource for #ident #ty_generics #data_source_where_clause {}
    };

    Ok(tokens)
}
