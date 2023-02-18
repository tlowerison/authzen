use convert_case::{Case, Casing};
use derive_more::IsVariant;
use proc_macro2::TokenStream;
use quote::{ToTokens, TokenStreamExt};
use std::ops::Deref;
use std::str::FromStr;
use syn::parse::{Error, Parse, ParseBuffer, ParseStream};
use syn::parse2;

#[derive(Clone, Debug)]
pub struct DynamicSchema {
    pub ident: syn::Ident,
    pub schema_relative_file_path: syn::LitStr,
}

#[derive(Clone, Debug)]
pub struct SchemaName(pub syn::Ident);

#[derive(Clone, Debug)]
pub struct TableName(pub syn::Ident);

#[derive(Clone, Debug)]
pub struct ColumnName(pub syn::Ident);

#[derive(Clone, Debug)]
pub struct ColumnSqlType(pub syn::Type);

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct Schema {
    pub name: SchemaName,
    pub tables: Vec<Table>,
    pub joinables: Vec<Joinable>,
    pub allow_tables_to_appear_in_same_query: Vec<TableName>,
}

#[derive(Clone, Debug)]
pub struct Table {
    pub name: TableName,
    pub primary_key: ColumnName,
    pub columns: Vec<Column>,
}

#[allow(unused)]
#[derive(Clone, Debug)]
pub struct Column {
    pub name: ColumnName,
    pub sql_type: ColumnSqlType,
    pub ty: syn::Path,
    pub is_nullable: bool,
    pub value_kind: ColumnValueKind,
}

#[derive(Clone, Debug, IsVariant)]
pub enum ColumnValueKind {
    None,
    Bool,
    Float,
    Int,
    String,
}

#[derive(Clone, Debug)]
pub struct Joinable {
    pub child_table: TableName,
    pub parent_table: TableName,
    pub child_table_foreign_key: ColumnName,
}

impl Parse for DynamicSchema {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let ident: syn::Ident = input.parse()?;
        let _comma: Token![,] = input.parse()?;
        let schema_relative_file_path: syn::LitStr = input.parse()?;
        Ok(Self {
            ident,
            schema_relative_file_path,
        })
    }
}

impl Parse for Schema {
    fn parse(parse_stream: ParseStream) -> syn::Result<Self> {
        let content: HasRef<'_, ParseBuffer>;
        let schema_name = SchemaName(
            if parse_stream.peek(Token![pub]) && parse_stream.peek2(Token![mod]) && parse_stream.peek3(syn::Ident) {
                let _pub: Token![pub] = parse_stream.parse()?;
                let _mod: Token![mod] = parse_stream.parse()?;
                let schema_name: syn::Ident = parse_stream.parse()?;
                let temp;
                braced!(temp in parse_stream);
                content = HasRef::Owned(temp);
                schema_name
            } else {
                content = HasRef::Borrowed(parse_stream);
                format_ident!("public")
            },
        );
        let parse_stream = content;

        let mut tables = Vec::<Table>::new();
        let mut joinables = Vec::<Joinable>::new();
        let mut allow_tables_to_appear_in_same_query = Vec::<TableName>::new();

        while !parse_stream.is_empty() {
            let macro_path: syn::Path = parse_stream.parse()?;
            let _macro_invocation: Token![!] = parse_stream.parse()?;
            let macro_name = &macro_path.segments.last().unwrap().ident;

            match &*macro_name.to_string() {
                "table" => {
                    let content;
                    braced!(content in parse_stream);
                    tables.push(content.parse()?);
                }
                "joinable" => {
                    let content;
                    parenthesized!(content in parse_stream);
                    let _semicolon: Token![;] = parse_stream.parse()?;
                    joinables.push(content.parse()?);
                }
                "allow_tables_to_appear_in_same_query" => {
                    let content;
                    parenthesized!(content in parse_stream);
                    while !content.is_empty() {
                        allow_tables_to_appear_in_same_query.push(TableName(content.parse()?));
                        let _comma: Token![,] = content.parse()?;
                    }
                    let _semicolon: Token![;] = parse_stream.parse()?;
                }
                _ => {
                    return Err(syn::Error::new_spanned(
                        macro_name,
                        format!("unable to parse diesel schema macros: unexpected macro `{macro_name}`"),
                    ))
                }
            };
        }

        Ok(Self {
            name: schema_name,
            tables,
            allow_tables_to_appear_in_same_query,
            joinables,
        })
    }
}

impl Parse for Table {
    fn parse(parse_stream: ParseStream) -> syn::Result<Self> {
        if parse_stream.peek2(Token![.]) {
            let _schema_name: syn::Ident = parse_stream.parse()?;
            let _dot: Token![.] = parse_stream.parse()?;
        }
        let table_name = TableName(parse_stream.parse()?);

        let content;
        parenthesized!(content in parse_stream);
        let primary_key = ColumnName(content.parse()?);

        let content;
        braced!(content in parse_stream);

        let mut columns = Vec::<Column>::new();

        while !content.is_empty() {
            let column_name = ColumnName(content.parse()?);
            let _arrow: Token![->] = content.parse()?;
            let column_sql_type = ColumnSqlType(content.parse()?);
            let _comma: Token![,] = content.parse()?;

            let ColumnSqlType(ty) = &column_sql_type;
            let mut ty_str = &*format!("{}", quote!(#ty));
            let is_nullable = ty_str.len() >= 8 && &ty_str[..8] == "Nullable";
            if is_nullable {
                ty_str = &ty_str[10..ty_str.len() - 1];
            }
            let value_kind = match &*ty_str.to_lowercase() {
                "bool" | "boolean" => ColumnValueKind::Bool,
                "bigint" | "bigserial" | "int" | "int2" | "int4" | "int8" | "integer" | "serial" | "serial2"
                | "serial4" | "serial8" | "smallInt" | "smallserial" => ColumnValueKind::Int,
                "decimal" | "doubleprecision" | "float4" | "float8" | "numeric" | "real" => ColumnValueKind::Float,
                "bytea" | "char" | "text" | "time" | "timetz" | "varchar" => ColumnValueKind::String,
                _ => ColumnValueKind::None,
            };

            columns.push(Column {
                ty: syn::Path {
                    leading_colon: None,
                    segments: syn::punctuated::Punctuated::from_iter(
                        [
                            syn::PathSegment {
                                ident: format_ident!("{}", table_name.0),
                                arguments: Default::default(),
                            },
                            syn::PathSegment {
                                ident: format_ident!("{}", column_name.0),
                                arguments: Default::default(),
                            },
                        ]
                        .into_iter(),
                    ),
                },
                name: column_name,
                sql_type: column_sql_type,
                is_nullable,
                value_kind,
            });
        }

        Ok(Self {
            name: table_name,
            primary_key,
            columns,
        })
    }
}

impl Parse for Joinable {
    fn parse(parse_stream: ParseStream) -> syn::Result<Self> {
        let child_table = TableName(parse_stream.parse()?);
        let _arrow: Token![->] = parse_stream.parse()?;
        let parent_table = TableName(parse_stream.parse()?);

        let content;
        parenthesized!(content in parse_stream);
        let child_table_foreign_key = ColumnName(content.parse()?);

        Ok(Self {
            child_table,
            parent_table,
            child_table_foreign_key,
        })
    }
}

impl ToTokens for Schema {
    fn to_tokens(&self, token_stream: &mut TokenStream) {
        let Schema {
            name,
            tables,
            joinables,
            ..
        } = self;
        let name = name.0.to_string();

        let joinable_tokens = joinables
            .iter()
            .map(|joinable| {
                let child_table = joinable.child_table.0.to_string();
                let parent_table = joinable.parent_table.0.to_string();
                quote!((
                    diesel_util::JoinablePair { child_table_name: #child_table, parent_table_name: #parent_table },
                    #joinable,
                ))
            })
            .collect::<Vec<_>>();
        let joinable_tokens = quote!(#(#joinable_tokens,)*);

        token_stream.append_all(quote! {
            diesel_util::DynamicSchema {
                name: #name,
                tables: std::collections::HashSet::from_iter([#(#tables,)*].into_iter()),
                joinables: std::collections::HashMap::from_iter([#joinable_tokens].into_iter()),
            }
        });
    }
}

impl ToTokens for Table {
    fn to_tokens(&self, token_stream: &mut TokenStream) {
        let Table {
            name,
            primary_key,
            columns,
        } = self;
        let name = name.0.to_string();
        let primary_key = primary_key.0.to_string();
        token_stream.append_all(quote! {
            diesel_util::Table {
                name: #name,
                primary_key: #primary_key,
                columns: std::collections::HashSet::from_iter([#(#columns,)*].into_iter()),
            }
        });
    }
}

impl ToTokens for Column {
    fn to_tokens(&self, token_stream: &mut TokenStream) {
        let Column { name, sql_type, .. } = self;
        let ColumnSqlType(ty) = sql_type;
        let name = name.0.to_string();
        let mut ty = quote! { #ty };
        let mut ty_str = &*format!("{ty}");
        let is_nullable = ty_str.len() >= 8 && &ty_str[..8] == "Nullable";
        if is_nullable {
            ty_str = &ty_str[10..ty_str.len() - 1];
        }
        ty = match &*ty_str.to_lowercase() {
            "smallInt" | "int2" | "smallserial" | "serial2" => quote! { SmallInt },
            "integer" | "int" | "int4" | "serial" | "serial4" => quote! { Integer },
            "bigint" | "int8" | "bigserial" | "serial8" => quote! { BigInt },
            "numeric" | "decimal" => quote! { Numeric },
            "real" | "float4" => quote! { Float },
            "doubleprecision" | "float8" => quote! { Double },
            "varchar" | "char" | "text" => quote! { Text },
            "bytea" => quote! { Binary },
            "time" | "timetz" => quote! { Time },
            "boolean" | "bool" => quote! { Bool },
            _ => TokenStream::from_str(ty_str).unwrap(),
        };
        let sql_type = if is_nullable {
            quote! { diesel_util::ColumnSqlType::IsNullable(std::sync::Arc::new(Box::new(Nullable::<#ty>::default()))) }
        } else {
            quote! { diesel_util::ColumnSqlType::NotNull(std::sync::Arc::new(Box::new(#ty::default()))) }
        };
        token_stream.append_all(quote! {
            diesel_util::Column {
                name: #name,
                sql_type: #sql_type,
            }
        });
    }
}

impl ToTokens for Joinable {
    fn to_tokens(&self, token_stream: &mut TokenStream) {
        let Joinable {
            child_table,
            parent_table,
            child_table_foreign_key,
        } = self;
        let child_table = child_table.0.to_string();
        let parent_table = parent_table.0.to_string();
        let child_table_foreign_key = child_table_foreign_key.0.to_string();
        token_stream.append_all(quote! {
            diesel_util::Joinable {
                child_table_name: #child_table,
                parent_table_name: #parent_table,
                child_table_foreign_key_column_name: #child_table_foreign_key,
            }
        });
    }
}

pub fn dynamic_schema(ident: syn::Ident, tokens: TokenStream) -> Result<TokenStream, Error> {
    let schema: Schema = parse2(tokens)?;

    let ident_snake = format_ident!("diesel_util_{}", format!("{ident}").to_case(Case::Snake));
    let ident_pascal = format_ident!("{}", format!("{ident}").to_case(Case::Pascal));

    let tokens = quote! {
        pub mod #ident_snake {
            use super::*;
            #[allow(unused_imports)]
            use diesel::sql_types::*;
            #[allow(unused_imports)]
            use diesel::ExpressionMethods;

            use diesel::backend::{Backend, DieselReserveSpecialization};
            use diesel::expression::BoxableExpression;
            use diesel::query_dsl::methods::{BoxedDsl, FilterDsl};

            #[derive(Clone, Debug)]
            pub struct #ident_pascal(pub diesel_util::DynamicSchema);

            impl std::ops::Deref for #ident_pascal {
                type Target = diesel_util::DynamicSchema;
                fn deref(&self) -> &Self::Target {
                    &self.0
                }
            }

            diesel_util::lazy_static::lazy_static! {
                pub static ref #ident: #ident_pascal = #ident_pascal(#schema);
            }
        }
        pub use #ident_snake::*;
    };

    Ok(tokens)
}

use util::*;
mod util {
    use super::*;

    pub(crate) enum HasRef<'a, T> {
        Borrowed(&'a T),
        Owned(T),
    }

    impl<T> From<T> for HasRef<'_, T> {
        fn from(t: T) -> Self {
            Self::Owned(t)
        }
    }

    impl<'a, T> From<&'a T> for HasRef<'a, T> {
        fn from(t: &'a T) -> Self {
            Self::Borrowed(t)
        }
    }

    impl<T> Deref for HasRef<'_, T> {
        type Target = T;
        fn deref(&self) -> &Self::Target {
            match self {
                Self::Borrowed(t) => t,
                Self::Owned(t) => t,
            }
        }
    }
}
