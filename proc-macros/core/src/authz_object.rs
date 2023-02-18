use proc_macro2::{Span, TokenStream};
use proc_macro_util::match_path;
use quote::quote;
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

    let tokens = quote! {
        impl<#lifetime> From<#inner_ty> for #ident<#lifetime> {
            fn from(value: #inner_ty) -> Self {
                Self(std::borrow::Cow::Owned(value))
            }
        }

        impl<#lifetime> From<&#lifetime #inner_ty> for #ident<#lifetime> {
            fn from(value: &#lifetime #inner_ty) -> Self {
                Self(std::borrow::Cow::Borrowed(value))
            }
        }

        impl<#lifetime> authzen::ObjectType for #ident<#lifetime> {
            const SERVICE: &'static str = #service;
            const TYPE: &'static str = #ty;
        }

        impl<#lifetime, Backend> authzen::AsStorage<Backend> for #ident<#lifetime>
        where
            #inner_ty: authzen::StorageObject<Backend>,
        {
            type Constructor<'__authz_object> = #ident<'__authz_object>;
            type StorageObject = #inner_ty;
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
