use either::{Either, Left, Right};
use proc_macro2::TokenStream;
use quote::TokenStreamExt;
use std::collections::HashMap;
use syn::parse::{Error, Parse, ParseStream};
use syn::parse2;
use syn::spanned::Spanned;
use uuid::Uuid;

#[derive(Debug)]
struct DbFilterParams {
    deserialize_ty: syn::Type,
    table_name: syn::Ident,
    db: syn::Ident,
    optional_params: DbFilterOptionalParams,
    is_static: bool,
}

#[derive(Debug, Default)]
struct DbFilterOptionalParams {
    columns: Option<DbFilterColumns>,
    distinct: bool,
    filter: Option<syn::Expr>,
    group_by: Option<syn::Expr>,
    inner_join: Option<DbFilterJoin>,
    left_join: Option<DbFilterJoin>,
    no_deleted_at: bool,
    order_by: Option<Vec<syn::Expr>>,
    page: Option<DbFilterPage>,
    pages: Option<DbFilterPage>,
    partition: Option<syn::Expr>,
    select: Option<syn::Expr>,
}

#[derive(Debug)]
enum ColumnComparison {
    Equals {
        operand: syn::Expr,
        operand_ty: Option<syn::Type>,
    },
    GreaterThan {
        operand: syn::Expr,
        operand_ty: Option<syn::Type>,
    },
    GreaterThanOrEqualTo {
        operand: syn::Expr,
        operand_ty: Option<syn::Type>,
    },
    IsNotNull {
        operand: Option<syn::Expr>,
        operand_ty: Option<syn::Type>,
    },
    IsNull {
        operand: Option<syn::Expr>,
        operand_ty: Option<syn::Type>,
    },
    LessThan {
        operand: syn::Expr,
        operand_ty: Option<syn::Type>,
    },
    LessThanOrEqualTo {
        operand: syn::Expr,
        operand_ty: Option<syn::Type>,
    },
    NotEquals {
        operand: syn::Expr,
        operand_ty: Option<syn::Type>,
    },
}

#[derive(Debug)]
struct DbFilterColumn {
    column_name: syn::Ident,
    comparison: ColumnComparison,
}

#[derive(Debug)]
struct DbFilterColumns<R = DbFilterColumn>(Vec<Either<DbFilterColumn, R>>);

trait DbFilterColumnsRight {
    fn is_right(fork: ParseStream) -> bool;
}

impl DbFilterColumnsRight for DbFilterColumn {
    fn is_right(_: ParseStream) -> bool {
        false
    }
}

/// DbFilterJoin has implicit joins, i.e. if joinable! is implemented between the main table
/// being filterd on and the listed table, then the listed table will be joined by what diesel
/// expects in the inner_join function
#[derive(Debug)]
struct DbFilterJoin(HashMap<syn::Ident, DbFilterColumns<DbFilterSubJoin>>);

#[derive(Debug)]
struct DbFilterSubJoin(JoinKind, Box<DbFilterJoin>);

impl DbFilterColumnsRight for DbFilterSubJoin {
    fn is_right(fork: ParseStream) -> bool {
        fork.parse::<JoinKind>().is_ok()
    }
}

#[derive(Debug)]
enum DbFilterPage {
    ImpliedIdent(syn::Ident, Option<syn::Type>),
    Expr(syn::Expr, Option<syn::Type>),
}

impl Parse for DbFilterParams {
    fn parse(parse_stream: ParseStream) -> syn::Result<Self> {
        let deserialize_ty: syn::Type = parse_stream.parse()?;
        let _comma: Token![,] = parse_stream.parse()?;

        let table_name: syn::Ident = parse_stream.parse()?;
        let _comma: Token![,] = parse_stream.parse()?;

        let db: syn::Ident = parse_stream.parse()?;

        if parse_stream.peek(Token![,]) {
            let _comma: Token![,] = parse_stream.parse()?;
        }

        let optional_params: DbFilterOptionalParams = if parse_stream.peek(syn::token::Brace) {
            let content;
            let _braces: syn::token::Brace = braced!(content in parse_stream);
            content.parse()?
        } else {
            Default::default()
        };

        if parse_stream.peek(Token![,]) {
            let _comma: Token![,] = parse_stream.parse()?;
        }

        let is_static = parse_stream.peek(Token![static]);
        if is_static {
            let _static: Token![static] = parse_stream.parse()?;
        }

        if parse_stream.peek(Token![,]) {
            let _comma: Token![,] = parse_stream.parse()?;
        }

        Ok(Self {
            deserialize_ty,
            table_name,
            db,
            optional_params,
            is_static,
        })
    }
}

impl Parse for DbFilterOptionalParams {
    fn parse(mut parse_stream: ParseStream) -> syn::Result<Self> {
        let mut columns: Option<DbFilterColumns> = None;
        let mut distinct = false;
        let mut filter: Option<syn::Expr> = None;
        let mut group_by: Option<syn::Expr> = None;
        let mut inner_join: Option<DbFilterJoin> = None;
        let mut left_join: Option<DbFilterJoin> = None;
        let mut no_deleted_at = false;
        let mut order_by: Option<Vec<syn::Expr>> = None;
        let mut page: Option<DbFilterPage> = None;
        let mut pages: Option<DbFilterPage> = None;
        let mut partition: Option<syn::Expr> = None;
        let mut select: Option<syn::Expr> = None;

        let mut first = true;
        while !parse_stream.is_empty() {
            if !first {
                let _comma: Token![,] = parse_stream.parse()?;
                if parse_stream.is_empty() {
                    break;
                }
            }
            let arg: syn::Ident = parse_stream.parse()?;

            match &*format!("{arg}") {
                "columns" => {
                    let _colon: Token![:] = parse_stream.parse()?;
                    let content;
                    let _brace: syn::token::Brace = braced!(content in parse_stream);
                    columns = Some(content.parse()?);
                },
                "distinct" => {
                    distinct = true;
                },
                "filter" => {
                    let _colon: Token![:] = parse_stream.parse()?;
                    filter = Some(parse_stream.parse()?);
                },
                "group_by" => {
                    let _colon: Token![:] = parse_stream.parse()?;
                    let expr: syn::Expr = parse_stream.parse()?;
                    group_by = Some(expr);
                },
                "inner_join"|"join" => {
                    let _colon: Token![:] = parse_stream.parse()?;
                    let content;
                    let _brace: syn::token::Brace = braced!(content in parse_stream);
                    let db_filter_join: DbFilterJoin = content.parse()?;
                    if let Some(inner_join) = inner_join.as_mut() {
                        inner_join.0.extend(db_filter_join.0.into_iter());
                    } else {
                        inner_join = Some(db_filter_join);
                    }
                },
                "left_join"|"left_outer_join" => {
                    let _colon: Token![:] = parse_stream.parse()?;
                    let content;
                    let _brace: syn::token::Brace = braced!(content in parse_stream);
                    let db_filter_join: DbFilterJoin = content.parse()?;
                    if let Some(left_join) = left_join.as_mut() {
                        left_join.0.extend(db_filter_join.0.into_iter());
                    } else {
                        left_join = Some(db_filter_join);
                    }
                },
                "no_deleted_at" => {
                    let _colon: Token![:] = parse_stream.parse()?;
                    let lit_bool: syn::LitBool = parse_stream.parse()?;
                    no_deleted_at = lit_bool.value;
                },
                "order_by" => {
                    let _colon: Token![:] = parse_stream.parse()?;
                    let content;
                    bracketed!(content in parse_stream);
                    let expr: syn::punctuated::Punctuated<syn::Expr, Token![,]> = content.parse_terminated(syn::Expr::parse)?;
                    order_by = Some(expr.into_iter().collect());
                },
                "page" => {
                    if parse_stream.peek(Token![:]) {
                        let _colon: Token![:] = parse_stream.parse()?;
                        let expr: syn::Expr = parse_stream.parse()?;
                        let operand_ty = get_operand_ty(&mut parse_stream)?;
                        page = Some(DbFilterPage::Expr(expr, operand_ty));
                    } else {
                        let operand_ty = get_operand_ty(&mut parse_stream)?;
                        page = Some(DbFilterPage::ImpliedIdent(arg, operand_ty));
                    }
                },
                "pages" => {
                    if parse_stream.peek(Token![:]) {
                        let _colon: Token![:] = parse_stream.parse()?;
                        let expr: syn::Expr = parse_stream.parse()?;
                        let operand_ty = get_operand_ty(&mut parse_stream)?;
                        pages = Some(DbFilterPage::Expr(expr, operand_ty));
                    } else {
                        let operand_ty = get_operand_ty(&mut parse_stream)?;
                        pages = Some(DbFilterPage::ImpliedIdent(arg, operand_ty));
                    }
                },
                "partition" => {
                    let _colon: Token![:] = parse_stream.parse()?;
                    let expr: syn::Expr = parse_stream.parse()?;
                    partition = Some(expr);
                },
                "select" => {
                    let _colon: Token![:] = parse_stream.parse()?;
                    let expr: syn::Expr = parse_stream.parse()?;
                    select = Some(expr);
                },
                _ => return Err(syn::Error::new_spanned(arg, "db_filter's optional params block only accepts fields `columns`,`filter`,`group_by`,`inner_join`,`join` (alias of `inner_join`),`no_deleted_at`,`left_join`,`left_outer_join`,`page`,`select`")),
            };

            first = false;
        }

        Ok(Self {
            columns,
            distinct,
            filter,
            group_by,
            inner_join,
            left_join,
            no_deleted_at,
            order_by,
            page,
            pages,
            partition,
            select,
        })
    }
}

impl Parse for DbFilterColumn {
    fn parse(parse_stream: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            column_name: parse_stream.parse()?,
            comparison: parse_stream.parse()?,
        })
    }
}

impl<R: DbFilterColumnsRight + Parse> Parse for DbFilterColumns<R> {
    fn parse(parse_stream: ParseStream) -> syn::Result<Self> {
        let mut columns: Vec<Either<DbFilterColumn, R>> = Default::default();

        let mut first = true;
        while !parse_stream.is_empty() {
            if !first {
                let _comma: Token![,] = parse_stream.parse()?;
                if parse_stream.is_empty() {
                    break;
                }
            }
            first = false;

            if R::is_right(&parse_stream.fork()) {
                columns.push(Right(parse_stream.parse()?));
            } else {
                columns.push(Left(parse_stream.parse()?));
            }
        }

        Ok(Self(columns))
    }
}

const INVALID_IDENT_COMPARISON_ERROR_MSG: &str =
    "unsupported column comparison keyword, did you mean `is null` or `is not null`?";

impl Parse for ColumnComparison {
    fn parse(mut parse_stream: ParseStream) -> syn::Result<Self> {
        if parse_stream.peek(Token![=]) {
            let _token: Token![=] = parse_stream.parse()?;
            let operand: syn::Expr = parse_stream.parse()?;
            let operand_ty = get_operand_ty(&mut parse_stream)?;
            Ok(Self::Equals { operand, operand_ty })
        } else if parse_stream.peek(Token![>]) && parse_stream.peek2(Token![=]) {
            let _token: Token![>=] = parse_stream.parse()?;
            let operand: syn::Expr = parse_stream.parse()?;
            let operand_ty = get_operand_ty(&mut parse_stream)?;
            Ok(Self::GreaterThanOrEqualTo { operand, operand_ty })
        } else if parse_stream.peek(Token![>]) {
            let _token: Token![>] = parse_stream.parse()?;
            let operand: syn::Expr = parse_stream.parse()?;
            let operand_ty = get_operand_ty(&mut parse_stream)?;
            Ok(Self::GreaterThan { operand, operand_ty })
        } else if parse_stream.peek(Token![<]) && parse_stream.peek2(Token![=]) {
            let _token: Token![<=] = parse_stream.parse()?;
            let operand: syn::Expr = parse_stream.parse()?;
            let operand_ty = get_operand_ty(&mut parse_stream)?;
            Ok(Self::LessThanOrEqualTo { operand, operand_ty })
        } else if parse_stream.peek(Token![<]) {
            let _token: Token![<] = parse_stream.parse()?;
            let operand: syn::Expr = parse_stream.parse()?;
            let operand_ty = get_operand_ty(&mut parse_stream)?;
            Ok(Self::LessThan { operand, operand_ty })
        } else if parse_stream.peek(Token![!]) && parse_stream.peek(Token![=]) {
            let _token: Token![!=] = parse_stream.parse()?;
            let operand: syn::Expr = parse_stream.parse()?;
            let operand_ty = get_operand_ty(&mut parse_stream)?;
            Ok(Self::NotEquals { operand, operand_ty })
        } else if parse_stream.peek(syn::Ident) {
            let is: syn::Ident = parse_stream.parse()?;
            if &*format!("{is}") != "is" {
                return Err(syn::Error::new(is.span(), INVALID_IDENT_COMPARISON_ERROR_MSG));
            }

            if parse_stream.peek(syn::Ident) && parse_stream.peek2(syn::Ident) {
                let not: syn::Ident = parse_stream.parse()?;
                let null: syn::Ident = parse_stream.parse()?;
                if &*format!("{not}") == "not" && &*format!("{null}") == "null" {
                    if parse_stream.peek(Token![if]) {
                        let _if: Token![if] = parse_stream.parse()?;
                        let operand: syn::Expr = parse_stream.parse()?;
                        let operand_ty = get_operand_ty(&mut parse_stream)?;
                        Ok(Self::IsNotNull {
                            operand: Some(operand),
                            operand_ty,
                        })
                    } else {
                        let operand_ty = get_operand_ty(&mut parse_stream)?;
                        Ok(Self::IsNotNull {
                            operand: None,
                            operand_ty,
                        })
                    }
                } else {
                    Err(syn::Error::new(
                        is.span().join(not.span()).unwrap().join(null.span()).unwrap(),
                        INVALID_IDENT_COMPARISON_ERROR_MSG,
                    ))
                }
            } else if parse_stream.peek(syn::Ident) {
                let null: syn::Ident = parse_stream.parse()?;
                if &*format!("{null}") == "null" {
                    if parse_stream.peek(Token![if]) {
                        let _if: Token![if] = parse_stream.parse()?;
                        let operand: syn::Expr = parse_stream.parse()?;
                        let operand_ty = get_operand_ty(&mut parse_stream)?;
                        Ok(Self::IsNull {
                            operand: Some(operand),
                            operand_ty,
                        })
                    } else {
                        let operand_ty = get_operand_ty(&mut parse_stream)?;
                        Ok(Self::IsNull {
                            operand: None,
                            operand_ty,
                        })
                    }
                } else {
                    Err(syn::Error::new(
                        is.span().join(null.span()).unwrap(),
                        INVALID_IDENT_COMPARISON_ERROR_MSG,
                    ))
                }
            } else {
                Err(parse_stream.error("unsupported column comparison"))
            }
        } else {
            Err(parse_stream.error("unsupported column comparison"))
        }
    }
}

fn get_operand_ty(parse_stream: &mut ParseStream) -> syn::Result<Option<syn::Type>> {
    if parse_stream.peek(Token![~]) {
        let _tilde: Token![~] = parse_stream.parse()?;
        Ok(Some(parse_stream.parse()?))
    } else {
        Ok(None)
    }
}

impl Parse for DbFilterJoin {
    fn parse(parse_stream: ParseStream) -> syn::Result<Self> {
        let mut join: HashMap<syn::Ident, DbFilterColumns<DbFilterSubJoin>> = Default::default();

        let mut first = true;
        while !parse_stream.is_empty() {
            if !first {
                let _comma: Token![,] = parse_stream.parse()?;
                if parse_stream.is_empty() {
                    break;
                }
            }
            let table_name: syn::Ident = parse_stream.parse()?;
            let _colon: Token![:] = parse_stream.parse()?;
            let content;
            let _brace: syn::token::Brace = braced!(content in parse_stream);
            let columns: DbFilterColumns<DbFilterSubJoin> = content.parse()?;

            join.insert(table_name, columns);

            first = false;
        }
        Ok(Self(join))
    }
}

impl Parse for DbFilterSubJoin {
    fn parse(parse_stream: ParseStream) -> syn::Result<Self> {
        let join_kind: JoinKind = parse_stream.parse()?;
        let _colon: Token![:] = parse_stream.parse()?;
        let content;
        braced!(content in parse_stream);
        Ok(Self(join_kind, Box::new(content.parse()?)))
    }
}

impl quote::ToTokens for DbFilterPage {
    fn to_tokens(&self, token_stream: &mut TokenStream) {
        let tokens = match self {
            Self::ImpliedIdent(ident, _) => quote! { #ident },
            Self::Expr(expr, _) => quote! { #expr },
        };
        for token in tokens.into_iter() {
            token_stream.append(token);
        }
    }
}

impl DbFilterPage {
    fn operand_ty(&self) -> Option<syn::Type> {
        match self {
            Self::ImpliedIdent(_, operand_ty) => operand_ty.clone(),
            Self::Expr(_, operand_ty) => operand_ty.clone(),
        }
    }
}

#[derive(Clone, Debug)]
struct Filter<'a> {
    clause: TokenStream,
    operand_expr: Option<&'a syn::Expr>,
    operand_ident: Option<syn::Ident>,
    operand_statement: TokenStream,
}

impl quote::ToTokens for ColumnComparison {
    fn to_tokens(&self, token_stream: &mut TokenStream) {
        let tokens = match self {
            Self::Equals { .. } => quote! { eq_any },
            Self::GreaterThan { .. } => quote! { gt },
            Self::GreaterThanOrEqualTo { .. } => quote! { ge },
            Self::IsNotNull { .. } => quote! { is_not_null },
            Self::IsNull { .. } => quote! { is_null },
            Self::LessThan { .. } => quote! { lt },
            Self::LessThanOrEqualTo { .. } => quote! { le },
            Self::NotEquals { .. } => quote! { ne_all },
        };
        for token in tokens.into_iter() {
            token_stream.append(token);
        }
    }
}

impl DbFilterColumn {
    fn filter<'a>(&'a self, table_name: &syn::Ident) -> Filter<'a> {
        let DbFilterColumn {
            column_name,
            comparison,
        } = self;
        let uuid = Uuid::new_v4(); // prevents collisions on identifier names when multiple filters use the column (# number columns in filter * 2 ** -48 chance of collision)
        let operand_ident = format_ident!("_{column_name}_operand_{}", uuid.as_u128() >> (128 - 48));

        let (comparison_call, operand_expr, operand_ty) = match comparison {
            ColumnComparison::IsNotNull { operand, operand_ty } => {
                (quote! { #comparison() }, operand.as_ref(), operand_ty.as_ref())
            }
            ColumnComparison::IsNull { operand, operand_ty } => {
                (quote! { #comparison() }, operand.as_ref(), operand_ty.as_ref())
            }
            ColumnComparison::Equals { operand, operand_ty } => (
                quote! { #comparison(#operand_ident.into_iter()) },
                Some(operand),
                operand_ty.as_ref(),
            ),
            ColumnComparison::GreaterThan { operand, operand_ty } => (
                quote! { #comparison(#operand_ident) },
                Some(operand),
                operand_ty.as_ref(),
            ),
            ColumnComparison::GreaterThanOrEqualTo { operand, operand_ty } => (
                quote! { #comparison(#operand_ident) },
                Some(operand),
                operand_ty.as_ref(),
            ),
            ColumnComparison::LessThan { operand, operand_ty } => (
                quote! { #comparison(#operand_ident) },
                Some(operand),
                operand_ty.as_ref(),
            ),
            ColumnComparison::LessThanOrEqualTo { operand, operand_ty } => (
                quote! { #comparison(#operand_ident) },
                Some(operand),
                operand_ty.as_ref(),
            ),
            ColumnComparison::NotEquals { operand, operand_ty } => (
                quote! { #comparison((&#operand_ident).into_iter()) },
                Some(operand),
                operand_ty.as_ref(),
            ),
        };

        let (operand_statement, operand_ident) = if let Some(operand_expr) = operand_expr.as_ref() {
            match comparison {
                ColumnComparison::IsNotNull { .. } | ColumnComparison::IsNull { .. } => (
                    quote! {
                        let #operand_ident: Option<bool> = (#operand_expr).into();
                        let #operand_ident: Option<()> = if #operand_ident.unwrap_or_default() { Some(()) } else { None };
                    },
                    None,
                ),
                _ => {
                    if let Some(operand_ty) = operand_ty.as_ref() {
                        (
                            quote! { let #operand_ident: Option<#operand_ty> = (#operand_expr).into(); },
                            Some(operand_ident),
                        )
                    } else {
                        (
                            quote! { let #operand_ident: Option<_> = (#operand_expr).into(); },
                            Some(operand_ident),
                        )
                    }
                }
            }
        } else {
            (quote! {}, None)
        };

        let clause = quote! { .filter(#table_name::#column_name.#comparison_call) };

        Filter {
            clause,
            operand_expr,
            operand_ident,
            operand_statement,
        }
    }
}

impl DbFilterColumns {
    fn filters<'a>(&'a self, table_name: &syn::Ident) -> Vec<Filter<'a>> {
        self.0
            .iter()
            .filter_map(|x| x.as_ref().left())
            .map(|db_filter_column| db_filter_column.filter(table_name))
            .collect()
    }
}

impl DbFilterColumns<DbFilterSubJoin> {
    fn filters<'a>(&'a self, table_name: &syn::Ident) -> Vec<Filter<'a>> {
        self.0
            .iter()
            .flat_map(|x| match x {
                Left(db_filter_column) => Left(std::iter::once(db_filter_column.filter(table_name))),
                Right(db_filter_sub_join) => Right(
                    db_filter_sub_join
                        .1
                         .0
                        .iter()
                        .flat_map(|(table_name, db_filter_columns)| db_filter_columns.filters(table_name)),
                ),
            })
            .collect()
    }
}

#[derive(Clone, Copy, Debug)]
enum JoinKind {
    Inner,
    Left,
}

impl Parse for JoinKind {
    fn parse(parse_stream: ParseStream) -> syn::Result<Self> {
        let join_kind: syn::Ident = parse_stream.parse()?;
        Ok(match &*join_kind.to_string() {
            "inner_join" | "join" => Self::Inner,
            "left_join" | "left_outer_join" => Self::Left,
            _ => {
                return Err(syn::parse::Error::new_spanned(
                    join_kind,
                    "expected one of: `join`, `inner_join`, `left_join`, `left_outer_join`",
                ))
            }
        })
    }
}

impl quote::ToTokens for JoinKind {
    fn to_tokens(&self, token_stream: &mut TokenStream) {
        let tokens = match self {
            Self::Inner => quote! { inner_join },
            Self::Left => quote! { left_join },
        };
        for token in tokens.into_iter() {
            token_stream.append(token);
        }
    }
}

impl DbFilterJoin {
    fn join_clauses(&self, join_kind: JoinKind) -> Vec<TokenStream> {
        self.0
            .iter()
            .map(|(table_name, columns)| {
                let sub_joins = columns
                    .0
                    .iter()
                    .filter_map(|x| x.as_ref().right())
                    .flat_map(|DbFilterSubJoin(sub_join_kind, sub_join)| {
                        sub_join.join_clauses(*sub_join_kind).into_iter()
                    })
                    .collect::<Vec<_>>();
                quote! { .#join_kind(#table_name::table #(#sub_joins)* ) }
            })
            .collect()
    }

    fn filter_deleted_at_clauses(&self) -> Vec<TokenStream> {
        self.0
            .keys()
            .map(|table_name| quote! { .filter(#table_name::deleted_at.is_null()) })
            .collect()
    }

    fn filters(&self) -> Vec<Filter<'_>> {
        self.0
            .iter()
            .flat_map(|(table_name, db_filter_columns)| db_filter_columns.filters(table_name).into_iter())
            .collect()
    }
}

pub fn db_filter(tokens: TokenStream) -> Result<TokenStream, Error> {
    let DbFilterParams {
        deserialize_ty,
        table_name,
        db,
        optional_params,
        is_static,
    } = parse2(tokens)?;
    let DbFilterOptionalParams {
        columns,
        distinct,
        filter,
        group_by,
        inner_join,
        left_join,
        no_deleted_at,
        order_by,
        page,
        pages,
        partition,
        select,
    } = optional_params;

    let inner_join_clauses = inner_join
        .as_ref()
        .map(|x| x.join_clauses(JoinKind::Inner))
        .unwrap_or_default();
    let left_join_clauses = left_join
        .as_ref()
        .map(|x| x.join_clauses(JoinKind::Left))
        .unwrap_or_default();

    let inner_join_filter_deleted_at_clauses = inner_join
        .as_ref()
        .map(|x| x.filter_deleted_at_clauses())
        .unwrap_or_default();
    let left_join_filter_deleted_at_clauses = left_join
        .as_ref()
        .map(|x| x.filter_deleted_at_clauses())
        .unwrap_or_default();

    let filter_deleted_at_clause = if no_deleted_at {
        quote! {}
    } else {
        quote! { .filter(#table_name::deleted_at.is_null()) }
    };

    let custom_filter_clause = if let Some(filter) = filter {
        quote! { #filter }
    } else {
        quote! {}
    };

    let select_clause = if let Some(select) = select {
        quote! { .select(#select) }
    } else {
        quote! { .select(#table_name::all_columns) }
    };
    let distinct_clause = if distinct {
        quote! { .distinct() }
    } else {
        quote! {}
    };

    let base_query = quote! {
        #table_name::table
            #distinct_clause

            #(#inner_join_clauses)*
            #(#left_join_clauses)*

            #filter_deleted_at_clause

            #(#inner_join_filter_deleted_at_clauses)*
            #(#left_join_filter_deleted_at_clauses)*

            #custom_filter_clause

            #select_clause
    };

    let base_query = if is_static {
        base_query
    } else {
        quote! { #base_query .into_boxed() }
    };

    let needs_group_by_clause =
        inner_join.as_ref().map(|x| x.0.len()).unwrap_or(0) + left_join.as_ref().map(|x| x.0.len()).unwrap_or(0) > 0;

    let group_by_clause = if needs_group_by_clause {
        if let Some(group_by) = group_by.as_ref() {
            quote! { .group_by(#group_by) }
        } else {
            quote! { .group_by(#table_name::id) }
        }
    } else {
        quote! {}
    };

    let order_by_clauses = if let Some(order_by) = order_by.as_ref() {
        if order_by.is_empty() {
            quote! {}
        } else {
            let order_by_expr = &order_by[0];
            let mut tokens = quote! { .order_by(#order_by_expr) };
            for order_by_expr in order_by.iter().skip(1) {
                tokens = quote! { .then_order_by(#order_by_expr) };
            }
            tokens
        }
    } else {
        quote! {}
    };

    let partition_clause = if let Some(partition) = partition {
        let str = quote! { #partition }.to_string();
        let str = str.trim();
        let str = str.trim_matches(|c| c == '(' || c == ')');
        let str = str.trim();
        for component in str.split(',') {
            if component.trim() == "" {
                continue;
            }
            let buf = component.split("::").collect::<Vec<_>>();
            if buf[buf.len() - 1].trim() == "id" && needs_group_by_clause {
                return Err(Error::new(partition.span(), "partitions on joined queries should not include columns named `id`, they can compile but will result in a runtime error due to ambiguity of column name if other columns are returned named `id`"));
            }
        }

        quote! { .partition(#partition) }
    } else if needs_group_by_clause && (page.is_some() || pages.is_some()) {
        if let Some(group_by) = group_by.as_ref() {
            quote! { .partition(#group_by) }
        } else {
            quote! { .partition((#table_name::id,)) }
        }
    } else {
        quote! {}
    };

    let mut column_filters = columns.as_ref().map(|x| x.filters(&table_name)).unwrap_or_default();

    let mut inner_join_column_filters = inner_join.as_ref().map(|x| x.filters()).unwrap_or_default();
    let mut left_join_column_filters = left_join.as_ref().map(|x| x.filters()).unwrap_or_default();

    let mut filters =
        Vec::with_capacity(column_filters.len() + inner_join_column_filters.len() + left_join_column_filters.len());
    filters.append(&mut column_filters);
    filters.append(&mut inner_join_column_filters);
    filters.append(&mut left_join_column_filters);

    let filter_operand_statements: Vec<_> = filters.iter().map(|filter| &filter.operand_statement).collect();
    let filter_operand_statements = quote! { #(#filter_operand_statements)* };

    let NestedQuery {
        multipaginated: query_multipaginated,
        non_paginated: query_non_paginated,
        paginated: query_paginated,
    } = if is_static {
        get_static_query(
            0,
            0,
            &deserialize_ty,
            &base_query,
            &filters,
            &group_by_clause,
            &order_by_clauses,
            &select_clause,
            &partition_clause,
        )
    } else {
        get_dynamic_query(
            &deserialize_ty,
            &base_query,
            &filters,
            &group_by_clause,
            &order_by_clauses,
            &partition_clause,
        )
    };

    let tokens = match pages {
        Some(pages) => {
            let operand_ty = match pages.operand_ty() {
                Some(operand_ty) => operand_ty,
                None => {
                    parse_quote!(&[&Page])
                }
            };
            quote! {
                async move {
                    #[allow(clippy::self_assignment)]
                    let db = #db;
                    #filter_operand_statements

                    if let Some(pages) = Into::<Option<#operand_ty>>::into(#pages) {
                        #query_multipaginated
                    } else {
                        #query_non_paginated
                    }
                }
            }
        }
        None => match page {
            Some(page) => {
                let operand_ty = match page.operand_ty() {
                    Some(operand_ty) => operand_ty,
                    None => parse_quote!(Page),
                };
                quote! {
                    async move {
                        #[allow(clippy::self_assignment)]
                        let db = #db;
                        #filter_operand_statements

                        if let Some(page) = Into::<Option<#operand_ty>>::into(#page) {
                            #query_paginated
                        } else {
                            #query_non_paginated
                        }
                    }
                }
            }
            None => quote! {
                async move {
                    #[allow(clippy::self_assignment)]
                    let db = #db;
                    #filter_operand_statements
                    #query_non_paginated
                }
            },
        },
    };

    Ok(tokens)
}

struct NestedQuery {
    multipaginated: TokenStream,
    non_paginated: TokenStream,
    paginated: TokenStream,
}

fn get_dynamic_query(
    deserialize_ty: &syn::Type,
    base_query: &TokenStream,
    filters: &Vec<Filter<'_>>,
    group_by_clause: &TokenStream,
    order_by_clauses: &TokenStream,
    partition_clause: &TokenStream,
) -> NestedQuery {
    let mut tokens = quote! {
        let mut query = #base_query;
    };

    for filter in filters {
        let Filter {
            operand_ident, clause, ..
        } = &filter;
        tokens = match operand_ident {
            Some(operand_ident) => quote! {
                #tokens
                if let Some(#operand_ident) = #operand_ident.as_ref() {
                    query = query #clause;
                }
            },
            None => quote! {
                #tokens
                query = query #clause;
            },
        };
    }

    NestedQuery {
        multipaginated: quote! {
            #tokens
            diesel_util::_Db::query(&db, move |conn| Box::pin(query
                #group_by_clause
                #order_by_clauses
                .multipaginate(pages.iter())
                #partition_clause
                .get_results::<#deserialize_ty>(conn)
            )).await
        },
        non_paginated: quote! {
            #tokens
            diesel_util::_Db::query(&db, move |conn| Box::pin(query
                #group_by_clause
                #order_by_clauses
                .get_results::<#deserialize_ty>(conn)
            )).await
        },
        paginated: quote! {
            #tokens
            diesel_util::_Db::query(&db, move |conn| Box::pin(query
                #group_by_clause
                #order_by_clauses
                .paginate(page)
                #partition_clause
                .get_results::<#deserialize_ty>(conn)
            )).await
        },
    }
}

#[allow(clippy::too_many_arguments)]
fn get_static_query(
    filter_index: usize,
    filter_index_set: u64,
    deserialize_ty: &syn::Type,
    base_query: &TokenStream,
    filters: &Vec<Filter<'_>>,
    group_by_clause: &TokenStream,
    order_by_clauses: &TokenStream,
    select_clause: &TokenStream,
    partition_clause: &TokenStream,
) -> NestedQuery {
    if filter_index == filters.len() {
        let filter_clauses: Vec<_> = filters
            .iter()
            .rev() // must reverse iterator since index set is ordered left to right against filter_index
            .enumerate()
            .filter_map(
                |(i, filter)| {
                    if (filter_index_set >> i) % 2 == 1 {
                        Some(&filter.clause)
                    } else {
                        None
                    }
                },
            )
            .collect();
        return NestedQuery {
            multipaginated: quote! {
                diesel_util::_Db::query(&db, move |conn| Box::pin(#base_query
                    #(#filter_clauses)*
                    #group_by_clause
                    #order_by_clauses
                    #select_clause
                    .multipaginate(pages.iter())
                    #partition_clause
                    .get_results::<#deserialize_ty>(conn)
                )).await
            },
            non_paginated: quote! {
                diesel_util::_Db::query(&db, move |conn| Box::pin(#base_query
                    #(#filter_clauses)*
                    #group_by_clause
                    #order_by_clauses
                    #select_clause
                    .get_results::<#deserialize_ty>(conn)
                )).await
            },
            paginated: quote! {
                diesel_util::_Db::query(&db, move |conn| Box::pin(#base_query
                    #(#filter_clauses)*
                    #group_by_clause
                    #order_by_clauses
                    #select_clause
                    .paginate(page)
                    #partition_clause
                    .get_results::<#deserialize_ty>(conn)
                )).await
            },
        };
    }

    if filters[filter_index].operand_expr.is_some() {
        let NestedQuery {
            multipaginated: nested_query_some_multipaginated,
            non_paginated: nested_query_some_non_paginated,
            paginated: nested_query_some_paginated,
        } = get_static_query(
            filter_index + 1,
            (filter_index_set << 1) + 1,
            deserialize_ty,
            base_query,
            filters,
            group_by_clause,
            order_by_clauses,
            select_clause,
            partition_clause,
        );
        let NestedQuery {
            multipaginated: nested_query_none_multipaginated,
            non_paginated: nested_query_none_non_paginated,
            paginated: nested_query_none_paginated,
        } = get_static_query(
            filter_index + 1,
            filter_index_set << 1,
            deserialize_ty,
            base_query,
            filters,
            group_by_clause,
            order_by_clauses,
            select_clause,
            partition_clause,
        );

        let Filter { operand_ident, .. } = &filters[filter_index];
        NestedQuery {
            multipaginated: quote! {
                if let Some(#operand_ident) = #operand_ident {
                    #nested_query_some_multipaginated
                } else {
                    #nested_query_none_multipaginated
                }
            },
            non_paginated: quote! {
                if let Some(#operand_ident) = #operand_ident {
                    #nested_query_some_non_paginated
                } else {
                    #nested_query_none_non_paginated
                }
            },
            paginated: quote! {
                if let Some(#operand_ident) = #operand_ident {
                    #nested_query_some_paginated
                } else {
                    #nested_query_none_paginated
                }
            },
        }
    } else {
        get_static_query(
            filter_index + 1,
            (filter_index_set << 1) + 1,
            deserialize_ty,
            base_query,
            filters,
            group_by_clause,
            order_by_clauses,
            select_clause,
            partition_clause,
        )
    }
}
