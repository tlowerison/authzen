use crate::util::{get_primary_key, DieselAttribute};
use itertools::Itertools;
use proc_macro2::TokenStream;
use syn::parse::{Error, Parse, ParseStream};
use syn::parse2;

#[derive(Default)]
struct AuditAttribute {
    struct_name: Option<syn::Ident>,
    table_name: Option<syn::Ident>,
    foreign_key: Option<syn::Ident>,
}

impl Parse for AuditAttribute {
    fn parse(parse_stream: ParseStream) -> syn::Result<Self> {
        let mut struct_name: Option<syn::Ident> = None;
        let mut table_name: Option<syn::Ident> = None;
        let mut foreign_key: Option<syn::Ident> = None;

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
            let ident: syn::Ident = parse_stream.parse()?;

            match &*format!("{arg}") {
                "struct_name" => struct_name = Some(ident),
                "table_name" => table_name = Some(ident),
                "foreign_key" => foreign_key = Some(ident),
                _ => return Err(syn::Error::new_spanned(
                    ident,
                    "audit expected one of `table_name`, `struct_name` or `foreign_key` to be passed as an argument",
                )),
            };
            first = false;
        }

        Ok(AuditAttribute {
            struct_name,
            table_name,
            foreign_key,
        })
    }
}

impl TryFrom<&syn::DeriveInput> for AuditAttribute {
    type Error = syn::parse::Error;
    fn try_from(ast: &syn::DeriveInput) -> Result<Self, Self::Error> {
        for attr in ast.attrs.iter() {
            if attr.path.is_ident("audit") {
                return attr.parse_args();
            }
        }
        Ok(Default::default())
    }
}

pub fn derive_audit(tokens: TokenStream) -> Result<TokenStream, Error> {
    let ast: syn::DeriveInput = parse2(tokens)?;

    let diesel_attribute = DieselAttribute::try_from(&ast).expect("Audit could not parse `diesel` attribute");
    let audit_attribute = AuditAttribute::try_from(&ast).expect("Audit could not parse `audit` attribute");

    let struct_name = ast.ident.clone();
    let table_name = diesel_attribute
        .table_name
        .expect("Audit was unable to extract table_name from a diesel(table_name = `...`) attribute");

    let audit_struct_name = audit_attribute
        .struct_name
        .unwrap_or_else(|| format_ident!("{}Audit", ast.ident));
    let audit_table_name = audit_attribute
        .table_name
        .unwrap_or_else(|| format_ident!("{table_name}_audit"));

    let primary_key_name = get_primary_key(&ast);

    let (mut field_names, mut field_types, mut field_attrs, mut field_vises): (Vec<_>, Vec<_>, Vec<_>, Vec<_>) =
        match ast.data {
            syn::Data::Struct(data_struct) => match data_struct.fields {
                syn::Fields::Named(fields_named) => fields_named
                    .named
                    .into_iter()
                    .map(|field| (field.ident.unwrap(), field.ty, field.attrs, field.vis))
                    .multiunzip(),
                _ => panic!("Audit can only be derived on struct data types with named fields at this time."),
            },
            _ => panic!("Audit can only be derived on struct data types at this time."),
        };

    let foreign_key = audit_attribute
        .foreign_key
        .unwrap_or_else(|| format_ident!("{table_name}_{primary_key_name}"));

    let mut primary_key_index: isize = -1;
    for (index, field_name) in field_names.iter().enumerate() {
        if *field_name == primary_key_name {
            primary_key_index = index as isize;
            break;
        }
    }

    let primary_key_index = if primary_key_index < 0 {
        panic!("Audit could not be derived for primary key `{primary_key_name}`, field not found in `{struct_name}`");
    } else {
        primary_key_index as usize
    };

    let primary_key_type = field_types[primary_key_index].clone();

    let mut into_field_names = field_names.clone();
    into_field_names.remove(primary_key_index);

    let vis = ast.vis;

    field_names.insert(primary_key_index + 1, foreign_key.clone());
    field_types.insert(primary_key_index + 1, primary_key_type);
    field_attrs.insert(primary_key_index + 1, Vec::default());
    field_vises.insert(primary_key_index + 1, vis.clone());

    let other_attributes = ast
        .attrs
        .into_iter()
        .filter(|attr| !attr.path.is_ident("audit") && !attr.path.is_ident("diesel"))
        .collect::<Vec<_>>();

    let tokens = quote! {
        #[derive(Associations, Clone, diesel_util::DieselUtilDerivative, Identifiable, Insertable, Queryable)]
        #[derivative(Debug)]
        #[diesel(
            table_name = #audit_table_name,
            primary_key(#primary_key_name),
            belongs_to(#struct_name, foreign_key = #foreign_key),
        )]
        #(#other_attributes)*
        #vis struct #audit_struct_name {
            #(
                #(#field_attrs)*
                #field_vises #field_names: #field_types,
            )*
        }

        impl From<#struct_name> for #audit_struct_name {
            fn from(record: #struct_name) -> Self {
                #audit_struct_name {
                    #primary_key_name: Uuid::new_v4(),
                    #foreign_key: record.#primary_key_name,
                    #(#into_field_names: record.#into_field_names,)*
                }
            }
        }

        impl diesel_util::Audit for #struct_name {
            type Raw = #audit_struct_name;
            type Table = #audit_struct_name;
        }
    };

    Ok(tokens)
}
