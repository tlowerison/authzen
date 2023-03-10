use authzen_proc_macro_util::{
    add_bounds_to_generics, find_field_attribute_in_struct, find_field_attributes_in_struct, MatchedAttribute,
};
use convert_case::{Case, Casing};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use std::borrow::Cow;
use syn::{parse2, parse_quote, Error};

pub fn context(item: TokenStream) -> Result<TokenStream, Error> {
    let ast: syn::DeriveInput = parse2(item)?;
    let ident = &ast.ident;

    let data_struct = match &ast.data {
        syn::Data::Struct(data_struct) => data_struct,
        _ => {
            return Err(Error::new_spanned(
                ast,
                "authzen::Context can only be derived on struct types".to_string(),
            ))
        }
    };
    let are_fields_named = matches!(data_struct.fields, syn::Fields::Named(_));

    let matched_subject_attribute = find_field_attribute_in_struct("subject", &ast)?;
    let mut matched_context_attributes = find_field_attributes_in_struct("context", &ast)?;

    if matched_context_attributes.len() > 1 {
        return Err(Error::new_spanned(
            &matched_context_attributes[1].attr,
            "`#[context]` attribute cannot be used more than once".to_string(),
        ));
    }

    let matched_context_attribute = matched_context_attributes.pop();

    let matched_authz_engine_attributes = find_field_attributes_in_struct("authz_engine", &ast)?;

    let matched_data_source_attributes = find_field_attributes_in_struct("data_source", &ast)?;

    let mut matched_transaction_cache_attributes = find_field_attributes_in_struct("transaction_cache", &ast)?;

    let mut transaction_cache_helpers = quote!();
    if matched_transaction_cache_attributes.is_empty() {
        let transaction_cache_ident =
            format_ident!("{}_TRANSACTION_CACHE", format!("{ident}").to_case(Case::ScreamingSnake));
        transaction_cache_helpers = quote!(
            pub static #transaction_cache_ident: () = ();
        );
        matched_transaction_cache_attributes.push(MatchedAttribute {
            attr: Cow::Owned(parse_quote!(#[transaction_cache])),
            field: Cow::Owned(syn::Field {
                attrs: Default::default(),
                vis: syn::Visibility::Inherited,
                ident: are_fields_named.then(|| format_ident!("__authzen_transaction_cache")),
                colon_token: are_fields_named.then(Default::default),
                ty: parse_quote!(()),
            }),
            field_accessor: quote!(&#transaction_cache_ident),
        });
    } else {
        for MatchedAttribute { field_accessor, .. } in &mut matched_transaction_cache_attributes {
            *field_accessor = quote!(&self.#field_accessor);
        }
    }

    let gat_lifetime: syn::Lifetime = parse_quote!('authzen_gat);

    let context_field_gat: syn::Type = matched_context_attribute
        .as_ref()
        .map(|x| {
            let field_ty = x.field.ty.clone();
            parse_quote!(&#gat_lifetime #field_ty)
        })
        .unwrap_or_else(|| parse_quote!(()));

    let context_field_access: TokenStream = matched_context_attribute
        .as_ref()
        .map(|MatchedAttribute { field_accessor, .. }| quote!(&self.#field_accessor))
        .unwrap_or_else(|| quote!(()));

    let subject_field_type = &matched_subject_attribute.field.ty;
    let subject_field_gat: syn::Type = parse_quote!(&#gat_lifetime #subject_field_type);

    let subject_field_accessor = &matched_subject_attribute.field_accessor;

    let authz_engine_field_types = matched_authz_engine_attributes
        .iter()
        .map(|MatchedAttribute { field, .. }| &field.ty)
        .collect::<Vec<_>>();
    let authz_engine_field_accessors = matched_authz_engine_attributes
        .iter()
        .map(|MatchedAttribute { field_accessor, .. }| field_accessor)
        .collect::<Vec<_>>();

    let data_source_field_types = matched_authz_engine_attributes
        .iter()
        .map(|_| {
            matched_data_source_attributes
                .iter()
                .map(move |MatchedAttribute { field, .. }| &field.ty)
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let data_source_field_accessors = matched_authz_engine_attributes
        .iter()
        .map(|_| {
            matched_data_source_attributes
                .iter()
                .map(move |MatchedAttribute { field_accessor, .. }| field_accessor)
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();

    let transaction_cache_field_types = matched_authz_engine_attributes
        .iter()
        .map(|_| {
            matched_data_source_attributes
                .iter()
                .map(|_| {
                    matched_transaction_cache_attributes
                        .iter()
                        .map(move |MatchedAttribute { field, .. }| &field.ty)
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let transaction_cache_field_accessors = matched_authz_engine_attributes
        .iter()
        .map(|_| {
            matched_data_source_attributes
                .iter()
                .map(|_| {
                    matched_transaction_cache_attributes
                        .iter()
                        .map(move |MatchedAttribute { field_accessor, .. }| field_accessor)
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();

    let (_, ty_generics, _) = ast.generics.split_for_impl();

    let mut trait_generics = ast.generics.clone();

    if let Some(MatchedAttribute { field, .. }) = matched_context_attribute {
        let context_field_type = &field.ty;
        add_bounds_to_generics(
            &mut trait_generics,
            [parse_quote!(Send), parse_quote!(Sync)],
            Some(&parse_quote!(#context_field_type)),
        );
    }
    add_bounds_to_generics(
        &mut trait_generics,
        [parse_quote!(Send), parse_quote!(Sync)],
        Some(&parse_quote!(#subject_field_type)),
    );

    let (impl_generics, _, where_clause) = trait_generics.split_for_impl();

    let tokens = quote! {
        #transaction_cache_helpers
        #(
            #(
                #(
                    impl #impl_generics authzen::AuthorizationContext<#authz_engine_field_types, #data_source_field_types, #transaction_cache_field_types> for #ident #ty_generics #where_clause {
                        type Context<#gat_lifetime> = #context_field_gat where Self: #gat_lifetime;
                        type Subject<#gat_lifetime> = #subject_field_gat where Self: #gat_lifetime;

                        fn context(&self) -> Self::Context<'_> {
                            #context_field_access
                        }
                        fn subject(&self) -> Self::Subject<'_> {
                            &self.#subject_field_accessor
                        }
                        fn authz_engine(&self) -> &#authz_engine_field_types {
                            &self.#authz_engine_field_accessors
                        }
                        fn data_source(&self) -> &#data_source_field_types {
                            &self.#data_source_field_accessors
                        }
                        fn transaction_cache(&self) -> &#transaction_cache_field_types {
                            #transaction_cache_field_accessors
                        }
                    }
                )*
            )*
        )*
    };

    Ok(tokens)
}
