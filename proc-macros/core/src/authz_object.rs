use proc_macro2::{Span, TokenStream};
use proc_macro_util::{add_general_bounds_to_generics, match_path};
use quote::{format_ident, quote};
use syn::parse::{Parse, ParseStream};
use syn::{parse2, parse_quote, punctuated::Punctuated, Error, Token};

#[derive(Clone, Debug)]
pub struct AuthzObjectArgs {
    pub service: String,
    pub ty: String,
}

pub fn authz_object(item: TokenStream) -> Result<TokenStream, Error> {
    let ast: syn::DeriveInput = parse2(item)?;
    let ident = &ast.ident;

    let field_type_path = match &ast.data {
        syn::Data::Struct(data_struct) => match &data_struct.fields {
            syn::Fields::Unnamed(fields) => {
                if fields.unnamed.len() != 1 {
                    return Err(Error::new_spanned(
                        ast,
                        "authzen::AuthzObject can only be derived on struct types with one unnamed field",
                    ));
                }
                match &fields.unnamed[0].ty {
                    syn::Type::Path(type_path) => &type_path.path,
                    _ => return Err(Error::new_spanned(
                        ast,
                        "authzen::AuthzObject can only be derived on struct types with one unnamed field which has a path type",
                    )),
                }
            }
            _ => {
                return Err(Error::new_spanned(
                    ast,
                    "authzen::AuthzObject can only be derived on struct types with one unnamed field",
                ))
            }
        },
        _ => {
            return Err(Error::new_spanned(
                ast,
                "authzen::AuthzObject can only be derived on struct types",
            ))
        }
    };

    let field_path_arguments = match &field_type_path.segments.last().unwrap().arguments {
        syn::PathArguments::AngleBracketed(args) => args,
        _ => {
            return Err(Error::new_spanned(
                field_type_path,
                "expected inner field to have type std::borrow::Cow",
            ))
        }
    };

    if field_path_arguments.args.len() != 2 {
        return Err(Error::new_spanned(
            field_type_path,
            "expected inner field to have type std::borrow::Cow",
        ));
    }

    let lifetime = match &field_path_arguments.args[0] {
        syn::GenericArgument::Lifetime(lifetime) => lifetime,
        _ => {
            return Err(Error::new_spanned(
                field_type_path,
                "expected inner field to have type std::borrow::Cow",
            ))
        }
    };

    let inner_ty = match &field_path_arguments.args[1] {
        syn::GenericArgument::Type(inner_ty) => inner_ty,
        _ => {
            return Err(Error::new_spanned(
                field_type_path,
                "expected inner field to have type std::borrow::Cow",
            ))
        }
    };

    match inner_ty {
        syn::Type::Path(type_path) => {
            let mut inner_ty_lifetime = None::<syn::Lifetime>;
            for path_segment in &type_path.path.segments {
                if let syn::PathArguments::AngleBracketed(generics) = &path_segment.arguments  {
                    for arg in &generics.args {
                        if let syn::GenericArgument::Lifetime(lt) = arg {
                            if inner_ty_lifetime.is_some() {
                                return Err(Error::new_spanned(lt, "authzen::AuthzObject currently can only be derived on types whose generic type parameter to Cow have at most one lifetime"));
                            }
                            inner_ty_lifetime = Some(lt.clone());
                        }
                    }
                }
            }
            inner_ty_lifetime.unwrap_or_else(|| parse_quote!('__authzen_authz_object_inner_ty))
        },
        _ => return Err(Error::new_spanned(
            inner_ty,
            "expected type argument to std::borrow::Cow to be a type path (i.e. not a reference, slice, etc. -- see https://docs.rs/syn/latest/syn/enum.Type.html#variant.Path)"
        )),
    };

    if match_path(&parse_quote!(std::borrow::Cow #field_path_arguments), field_type_path).is_err() {
        return Err(Error::new_spanned(
            field_type_path,
            "expected inner field to have type std::borrow::Cow",
        ));
    }

    let attr = ast.attrs
        .iter()
        .find(|attr| attr.path.is_ident("authzen"))
        .ok_or_else(|| Error::new_spanned(&ast, "expected an attribute specifying the `service` and `ty` arguments, whose values are literal strings representing the associated consts `authzen::AuthzObject::SERVICE` and `authzen::AuthzObject::TYPE`"))?;

    let AuthzObjectArgs { service, ty } = attr.parse_args()?;

    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();

    let mut identifiable_generics = ast.generics.clone();
    add_general_bounds_to_generics(
        &mut identifiable_generics,
        [parse_quote!(#inner_ty: authzen::Identifiable)],
    );
    let (_, _, identifiable_where_clause) = identifiable_generics.split_for_impl();

    let backend_ident = format_ident!("Backend");
    let mut as_storage_generics = ast.generics.clone();
    as_storage_generics.params.push(parse_quote!(#backend_ident));
    add_general_bounds_to_generics(
        &mut as_storage_generics,
        [parse_quote!(#inner_ty: authzen::StorageObject<Backend>)],
    );
    let (as_storage_impl_generics, _, as_storage_where_clause) = as_storage_generics.split_for_impl();

    let tokens = quote! {
        impl #impl_generics From<#inner_ty> for #ident #ty_generics #where_clause {
            fn from(value: #inner_ty) -> Self {
                Self(std::borrow::Cow::Owned(value))
            }
        }

        impl #impl_generics From<&#lifetime #inner_ty> for #ident #ty_generics #where_clause {
            fn from(value: &#lifetime #inner_ty) -> Self {
                Self(std::borrow::Cow::Borrowed(value))
            }
        }

        impl #impl_generics authzen::ObjectType for #ident #ty_generics #where_clause {
            const SERVICE: &'static str = #service;
            const TYPE: &'static str = #ty;
        }

        impl #as_storage_impl_generics authzen::AsStorage<#backend_ident> for #ident #ty_generics #as_storage_where_clause {
            type Constructor<'__authzen_authz_object> = #ident<'__authzen_authz_object>;
            type StorageObject = #inner_ty;
        }

        impl #impl_generics authzen::Identifiable for #ident #ty_generics #identifiable_where_clause {
            type Id = <#inner_ty as Identifiable>::Id;
            fn id(&self) -> &Self::Id {
                use std::ops::Deref;
                self.deref().id()
            }
        }
    };

    Ok(tokens)
}

impl Parse for AuthzObjectArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let args = Punctuated::<AuthzObjectArg, Token![,]>::parse_terminated(input)?;
        if args.len() != 2 {
            return Err(Error::new(Span::call_site(), "expected both args: `service` and `ty`"));
        }
        let mut service = None::<String>;
        let mut ty = None::<String>;

        for arg in args {
            match arg {
                AuthzObjectArg::Service(service_str) => service = Some(service_str),
                AuthzObjectArg::Type(ty_str) => ty = Some(ty_str),
            }
        }

        Ok(Self {
            service: service.ok_or_else(|| Error::new(Span::call_site(), "missing `service` argument"))?,
            ty: ty.ok_or_else(|| Error::new(Span::call_site(), "missing `ty` argument"))?,
        })
    }
}

#[derive(Clone, Debug)]
enum AuthzObjectArg {
    Service(String),
    Type(String),
}

impl Parse for AuthzObjectArg {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let ident: syn::Ident = input.parse()?;
        let _: Token![=] = input.parse()?;

        match &*ident.to_string() {
            "service" => Ok(Self::Service(input.parse::<syn::LitStr>()?.value())),
            "ty" => Ok(Self::Type(input.parse::<syn::LitStr>()?.value())),
            _ => Err(Error::new_spanned(
                ident,
                "unrecognized argument, expected `service` or `ty`".to_string(),
            )),
        }
    }
}
