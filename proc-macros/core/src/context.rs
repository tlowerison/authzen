use proc_macro2::TokenStream;
use proc_macro_util::{
    add_bounds_to_generics, find_field_attribute_in_struct, find_field_attributes_in_struct, MatchedAttribute,
};
use quote::quote;
use syn::{parse2, parse_quote, Error};

pub fn context(item: TokenStream) -> Result<TokenStream, Error> {
    let ast: syn::DeriveInput = parse2(item)?;
    let ident = &ast.ident;

    match &ast.data {
        syn::Data::Struct(_) => {}
        _ => {
            return Err(Error::new_spanned(
                ast,
                "authzen::Context can only be derived on struct types".to_string(),
            ))
        }
    };

    let matched_subject_attribute = find_field_attribute_in_struct("subject", &ast)?;
    let mut matched_context_attributes = find_field_attributes_in_struct("context", &ast)?;

    if matched_context_attributes.len() > 1 {
        return Err(Error::new_spanned(
            matched_context_attributes[1].attr,
            "`#[context]` attribute cannot be used more than once".to_string(),
        ));
    }

    let matched_context_attribute = matched_context_attributes.pop();

    let matched_decision_maker_attributes = find_field_attributes_in_struct("decision_maker", &ast)?;

    let matched_storage_client_attributes = find_field_attributes_in_struct("storage_client", &ast)?;

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

    let decision_maker_field_types = matched_decision_maker_attributes
        .iter()
        .map(|MatchedAttribute { field, .. }| &field.ty)
        .collect::<Vec<_>>();
    let decision_maker_field_accessors = matched_decision_maker_attributes
        .iter()
        .map(|MatchedAttribute { field_accessor, .. }| field_accessor)
        .collect::<Vec<_>>();

    let storage_client_field_types = matched_decision_maker_attributes
        .iter()
        .map(|_| {
            matched_storage_client_attributes
                .iter()
                .map(move |MatchedAttribute { field, .. }| &field.ty)
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let storage_client_field_accessors = matched_decision_maker_attributes
        .iter()
        .map(|_| {
            matched_storage_client_attributes
                .iter()
                .map(move |MatchedAttribute { field_accessor, .. }| field_accessor)
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
        #(
            impl #impl_generics authzen::CanContext<#decision_maker_field_types> for #ident #ty_generics #where_clause {
                type Context<#gat_lifetime> = #context_field_gat where Self: #gat_lifetime;
                type Subject<#gat_lifetime> = #subject_field_gat where Self: #gat_lifetime;

                fn context(&self) -> Self::Context<'_> {
                    #context_field_access
                }
                fn subject(&self) -> Self::Subject<'_> {
                    &self.#subject_field_accessor
                }
                fn decision_maker(&self) -> &#decision_maker_field_types {
                    &self.#decision_maker_field_accessors
                }
            }
            #(
                impl #impl_generics authzen::TryContext<#decision_maker_field_types, #storage_client_field_types> for #ident #ty_generics #where_clause {
                    fn storage_client(&self) -> &#storage_client_field_types {
                        &self.#storage_client_field_accessors
                    }
                }
            )*
        )*
    };

    Ok(tokens)
}
