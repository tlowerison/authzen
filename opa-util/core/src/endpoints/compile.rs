use crate::models::*;
use crate::OPAClient;
use futures::future::try_join_all;
use hyper::{body::Bytes, http::header::*, Body, Method};
use serde::de::{Deserialize, Deserializer, Error};
use serde_json::Value;
use service_util::*;
use std::borrow::Cow;
use std::cmp::{Ord, Ordering, PartialOrd};
use std::collections::{HashMap, HashSet};
use std::fmt::{self, Debug};

// NOTE: PE used in the types found in this module stands for Partially Evaluated

type PEUnknowns = &'static [&'static str];

const DEFAULT_QUERY: &str = "data.app.authz";
const FETCH: &str = "data.util.fetch";

static PE_UNKNOWNS_READ: [&str; 1] = ["input.action.object.ids"];
static DISABLE_INLINING: [&str; 1] = [FETCH];

#[derive(Clone, Debug, Serialize, TypedBuilder)]
#[builder(field_defaults(default, setter(into)))]
pub struct PEQuery<'a> {
    #[builder(default = Cow::Borrowed(DEFAULT_QUERY))]
    pub query: Cow<'a, str>,
    #[builder(!default)]
    pub input: PEQueryInput<'a>,
    #[serde(skip_serializing)]
    pub explain: Option<&'a str>,
    #[serde(skip_serializing)]
    pub pretty: Option<bool>,
    #[serde(skip_serializing)]
    pub metrics: Option<bool>,
    #[serde(skip_serializing)]
    pub instrument: Option<bool>,
}

#[derive(Clone, Derivative, Deserialize, Serialize, TypedBuilder)]
#[derivative(Debug)]
pub struct PEQueryInput<'a> {
    #[builder(setter(into))]
    pub action: PEQueryInputAction<'a>,
    #[derivative(Debug = "ignore")]
    pub token: Option<&'a str>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case", tag = "type", content = "object")]
pub enum PEQueryInputAction<'a> {
    Read {
        service: Cow<'a, str>,
        entity: Cow<'a, str>,
    },
}

impl PEQueryInputAction<'_> {
    fn service(&self) -> &str {
        match self {
            Self::Read { service, .. } => service,
        }
    }
    fn entity(&self) -> &str {
        match self {
            Self::Read { entity, .. } => entity,
        }
    }
    fn unknowns(&self) -> PEUnknowns {
        match self {
            Self::Read { .. } => &PE_UNKNOWNS_READ,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[skip_serializing_none]
pub struct PEQueryParams<'a> {
    explain: Option<&'a str>,
    pretty: Option<bool>,
    metrics: Option<bool>,
    instrument: Option<bool>,
}

#[derive(Clone, Debug, Serialize)]
struct PEQueryBody<'a> {
    query: &'a Cow<'a, str>,
    input: &'a PEQueryInput<'a>,
    unknowns: PEUnknowns,
    options: PEQueryBodyOptions,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PEQueryBodyOptions {
    disable_inlining: &'static [&'static str],
}

impl<'a> From<&'a PEQuery<'_>> for PEQueryParams<'a> {
    fn from(pe_query: &'a PEQuery<'_>) -> Self {
        Self {
            explain: pe_query.explain,
            pretty: pe_query.pretty,
            metrics: pe_query.metrics,
            instrument: pe_query.instrument,
        }
    }
}

impl<'a> From<&'a PEQuery<'_>> for PEQueryBody<'a> {
    fn from(pe_query: &'a PEQuery<'_>) -> Self {
        Self {
            query: &pe_query.query,
            input: &pe_query.input,
            unknowns: pe_query.input.action.unknowns(),
            options: PEQueryBodyOptions {
                disable_inlining: &DISABLE_INLINING,
            },
        }
    }
}

impl Endpoint for PEQuery<'_> {
    const METHOD: Method = Method::POST;

    type Params<'a> = PEQueryParams<'a> where Self: 'a;

    fn path(&self) -> Path {
        "/v1/compile".into()
    }
    fn params(&self) -> Self::Params<'_> {
        self.into()
    }
    fn headers(&self) -> HeaderMap {
        HeaderMap::from_iter(vec![(CONTENT_TYPE, HeaderValue::from_str("application/json").unwrap())])
    }
    fn body(&self) -> Body {
        let body = PEQueryBody::from(self);
        let body = serde_json::to_string(&body).unwrap();
        if *crate::OPA_DEBUG {
            info!("OPA Request: {}", self.path());
            info!("{body}");
        }
        Body::from(Bytes::copy_from_slice(body.as_bytes()))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Evaluation {
    Accept,
    Filter(Vec<Filter>),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Filter {
    Equality {
        negated: bool,
        left: FilterVar,
        right: FilterVar,
    },
    Exists {
        negated: bool,
        key: FilterVar,
    },
    In {
        negated: bool,
        key: FilterVar,
        set: FilterVar,
    },
}

#[derive(Clone, Debug, Deref, Eq, PartialEq)]
pub struct AllOver(pub Vec<Value>);

#[derive(Clone, Debug, Deref, Eq, PartialEq)]
pub struct AnyOver(pub Box<FilterVar>);

#[derive(Clone, Debug, Deref, Eq, PartialEq)]
pub struct Unknown(pub Vec<String>);

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FilterVar {
    AllOver(AllOver),
    AnyOver(AnyOver),
    Unknown(Unknown),
    Value(Value),
}

enum FilterVarEquality<'a> {
    AllOverAllOver(&'a AllOver, &'a AllOver),
    AllOverAnyOver(&'a AllOver, &'a AnyOver),
    AllOverUnknown(&'a AllOver, &'a Unknown),
    AllOverValue(&'a AllOver, &'a Value),
    AnyOverAllOver(&'a AnyOver, &'a AllOver),
    AnyOverAnyOver(&'a AnyOver, &'a FilterVar),
    AnyOverUnknown(&'a AnyOver, &'a Unknown),
    AnyOverValue(&'a AnyOver, &'a Value),
    UnknownUnknown(&'a Unknown, &'a Unknown),
    UnknownValue(&'a Unknown, &'a Value),
    ValueValue(&'a Value, &'a Value),
}

impl<'a> From<(&'a FilterVar, &'a FilterVar)> for FilterVarEquality<'a> {
    fn from((left, right): (&'a FilterVar, &'a FilterVar)) -> Self {
        match left {
            FilterVar::AllOver(all_over) => match right {
                FilterVar::AllOver(all_over2) => Self::AllOverAllOver(all_over, all_over2),
                FilterVar::AnyOver(any_over) => Self::AllOverAnyOver(all_over, any_over),
                FilterVar::Unknown(unknown) => Self::AllOverUnknown(all_over, unknown),
                FilterVar::Value(value) => Self::AllOverValue(all_over, value),
            },
            FilterVar::AnyOver(any_over) => match right {
                FilterVar::AllOver(all_over) => Self::AnyOverAllOver(any_over, all_over),
                FilterVar::AnyOver(any_over2) => Self::AnyOverAnyOver(any_over, any_over2),
                FilterVar::Unknown(unknown) => Self::AnyOverUnknown(any_over, unknown),
                FilterVar::Value(value) => Self::AnyOverValue(any_over, value),
            },
            FilterVar::Unknown(unknown) => match right {
                FilterVar::AllOver(all_over) => Self::AllOverUnknown(all_over, unknown),
                FilterVar::AnyOver(any_over) => Self::AnyOverUnknown(any_over, unknown),
                FilterVar::Unknown(unknown2) => Self::UnknownUnknown(unknown, unknown2),
                FilterVar::Value(value) => Self::UnknownValue(unknown, value),
            },
            FilterVar::Value(value) => match right {
                FilterVar::AllOver(all_over) => Self::AllOverValue(all_over, value),
                FilterVar::AnyOver(any_over) => Self::AnyOverValue(any_over, value),
                FilterVar::Unknown(unknown) => Self::UnknownValue(unknown, value),
                FilterVar::Value(value2) => Self::ValueValue(value, value2),
            },
        }
    }
}

pub use evaluation::*;
mod evaluation {
    use super::*;

    pub async fn pe_query(
        opa_client: &OPAClient,
        query: impl Into<PEQuery<'_>>,
        data_cache: impl Into<Option<Value>>,
    ) -> Result<Evaluation, anyhow::Error> {
        let query = query.into();
        // TODO: remove clone
        let input = query.input.clone();
        let response = query.raw().query(opa_client).await?;
        let bytes = response.body();

        let value: Value = serde_json::from_slice(bytes).map_err(|err| {
            anyhow::Error::msg(format!(
                "{err}:\n{}",
                serde_json::to_string_pretty(&serde_json::from_slice::<Value>(bytes).unwrap()).unwrap()
            ))
        })?;

        info!("RESPONSE");
        info!("{value}");

        let PEQueryResponse { result } = serde_json::from_slice(bytes).map_err(|err| {
            anyhow::Error::msg(format!(
                "{err}:\n{}",
                serde_json::to_string_pretty(&serde_json::from_slice::<Value>(bytes).unwrap()).unwrap()
            ))
        })?;

        let mut queries = result.queries.ok_or_else(|| anyhow::Error::msg("Unauthorized"))?;
        if queries.is_empty() {
            return Err(anyhow::Error::msg("Unauthorized"));
        }
        if queries.len() > 1 {
            return Err(anyhow::Error::msg(
                "Unauthorized: unsupported number of queries returned",
            ));
        }
        if queries[0].0.len() != 1 {
            return Err(anyhow::Error::msg(
                "Unauthorized: unsupported number of queries returned",
            ));
        }

        let mut query = queries.pop().unwrap().0.pop().unwrap();
        if query.terms.len() != 1 {
            return Err(anyhow::Error::msg(
                "Unauthorized: unsupported number of query terms returned",
            ));
        }

        let query_term = query.terms.pop().unwrap();
        let query_term_ref_nodes = if let OPAPolicyASTNode::Ref(OPAPolicyASTNodeRef(opa_policy_path_nodes)) = query_term
        {
            opa_policy_path_nodes
        } else {
            return Err(anyhow::Error::msg(format!(
                "Unauthorized: unexpected query term returned: {query_term:?}"
            )));
        };
        if query_term_ref_nodes.len() != 4 {
            return Err(anyhow::Error::msg(format!(
                "Unauthorized: unexpected number of query term ref nodes returned: {query_term_ref_nodes:?}"
            )));
        }
        match &query_term_ref_nodes[0] {
            OPAPolicyPathNode::Var("data") => {}
            node => {
                return Err(anyhow::Error::msg(format!(
                    "Unauthorized: unexpected query term ref node[0]: {node:?}"
                )))
            }
        }
        match &query_term_ref_nodes[1] {
            OPAPolicyPathNode::String("partial") => {}
            node => {
                return Err(anyhow::Error::msg(format!(
                    "Unauthorized: unexpected query term ref node[1]: {node:?}"
                )))
            }
        }
        match &query_term_ref_nodes[2] {
            OPAPolicyPathNode::String("app") => {}
            node => {
                return Err(anyhow::Error::msg(format!(
                    "Unauthorized: unexpected query term ref node[2]: {node:?}"
                )))
            }
        }
        match &query_term_ref_nodes[3] {
            OPAPolicyPathNode::String("authz") => {}
            node => {
                return Err(anyhow::Error::msg(format!(
                    "Unauthorized: unexpected query term ref node[3]: {node:?}"
                )))
            }
        }

        let mut supports = result.supports.ok_or_else(|| anyhow::Error::msg("Unauthorized"))?;
        if supports.len() != 1 {
            return Err(anyhow::Error::msg("Unauthorized: unexpected number of supports"));
        }

        let PEQueryResponseResultSupport { package, mut rules } = supports.pop().unwrap();
        if package.paths.len() != 3 {
            return Err(anyhow::Error::msg(
                "Unauthorized: unexpected number of support.package.paths",
            ));
        }
        match &package.paths[0] {
            OPAPolicyPathNode::Var("data") => {}
            node => {
                return Err(anyhow::Error::msg(format!(
                    "Unauthorized: unexpected query term ref node[0]: {node:?}"
                )))
            }
        }
        match &package.paths[1] {
            OPAPolicyPathNode::String("partial") => {}
            node => {
                return Err(anyhow::Error::msg(format!(
                    "Unauthorized: unexpected query term ref node[1]: {node:?}"
                )))
            }
        }
        match &package.paths[2] {
            OPAPolicyPathNode::String("app") => {}
            node => {
                return Err(anyhow::Error::msg(format!(
                    "Unauthorized: unexpected query term ref node[2]: {node:?}"
                )))
            }
        }

        let rules = rules
            .iter_mut()
            .filter_map(|rule| {
                if rule.head.name != "authz" {
                    return None;
                }
                let positive = match rule.head.kind {
                    PEQueryResponseResultSupportRuleHeadKind::Value(OPAPolicyASTNode::Boolean(is_rule_positive)) => {
                        is_rule_positive
                    }
                    _ => return None,
                };

                rule.conditions.sort();

                let mut operations = Vec::<Operation<'_, '_>>::new();
                for condition in &rule.conditions {
                    let operation = match condition.as_operation() {
                        Ok(operation) => operation,
                        Err(err) => return Some(Err(anyhow::Error::msg(format!("Unauthorized: {err}")))),
                    };
                    match &operation {
                        Operation::Boolean(_) => {} // ignore Boolean variants for now
                        _ => operations.push(operation),
                    };
                }
                operations.sort();

                Some(Ok::<_, anyhow::Error>(Rule { positive, operations }))
            })
            .collect::<Result<Vec<_>, _>>()?;

        let mut var_scope = VarScope::new();
        let input_json = serde_json::to_value(input.clone()).unwrap();
        add_to_scope(
            &mut var_scope,
            &input_json,
            &OPAPolicyASTNodeRef(vec![]),
            OPAPolicyPathNode::Var("input"),
        );

        let data_cache = data_cache.into();
        if let Some(data_cache) = data_cache.as_ref() {
            add_to_scope(
                &mut var_scope,
                data_cache,
                &OPAPolicyASTNodeRef(vec![]),
                OPAPolicyPathNode::Var("data"),
            );
        }

        let unknowns: HashSet<&str> = input.action.unknowns().iter().copied().collect();
        let authz = try_join_all(rules.iter().map(|rule| {
            eval_operations(
                &rule.operations,
                rule.positive,
                opa_client,
                &unknowns,
                &input.action,
                var_scope.clone(),
            )
        }))
        .await?;

        if *crate::OPA_DEBUG {
            info!("{authz:#?}");
        }

        let authz = authz.into_iter().fold(_Evaluation::Reject, |acc, eval| acc.or(eval));

        match authz {
            _Evaluation::Accept => Ok(Evaluation::Accept),
            _Evaluation::Filter(filters) => Ok(Evaluation::Filter(filters)),
            _Evaluation::Reject => Err(anyhow::Error::msg("Unauthorized")),
        }
    }

    // cfg_if! {
    //     if #[cfg(feature = "db-filter")] {
    //         use diesel_dynamic_schema::*;
    //         use diesel_util::DynamicSchema;
    //         use diesel::expression::{expression_types::NotSelectable, SqlLiteral};
    //         use diesel::expression_methods::ExpressionMethods;
    //         use diesel::sql_types::Bool;

    //         pub type SchemasByService = HashMap<&'static str, DynamicSchema>;

    //         pub type JoinSet<'a> = HashSet<&'a str>;

    //         #[derive(Clone, Copy, Debug)]
    //         pub enum PEResult {
    //             Accept,
    //             Filters(PEFilters),
    //             Reject,
    //         }

    //         #[derive(Clone, Debug)]
    //         pub struct PEFilters {
    //             pub where_clause: Option<SqlLiteral<Bool>>,
    //             pub joins: SqlLiteral<NonSelectable>,
    //             pub external_service_queries: Vec<SqlQuery>,
    //         }

    //         #[derive(Clone, Debug)]
    //         pub struct PEExternalFilters {
    //             pub service: &'static str,
    //             pub entity: &'static str,
    //             pub where_clause: Option<SqlLiteral<Bool>>,
    //             pub joins: SqlLiteral<NonSelectable>,
    //         }

    //         impl Filter {
    //             pub fn get_query(&self, join_set: &mut JoinSet<'_>) -> Result<PEResult, anyhow::Error> {
    //                 match self {
    //                     Self::Equality { negated, left, right } => FilterVarEquality::from((left, right)).get_query(query, schemas, schema, join_set),
    //                     Self::Exists { negated, key } => todo!(),
    //                     Self::In { negated, key, set } => todo!(),
    //                 }
    //             }
    //         }

    //         impl<'a> FilterVarEquality<'a> {
    //             pub fn get_query(&self, join_set: &mut JoinSet<'a>) -> Result<PEResult<SqlLiteral<Bool>>, anyhow::Error> {
    //                 match self {
    //                     // kind of a weird pair, not sure if this would ever exist
    //                     // every item1 in all_over1 {
    //                     //     every item2 in all_over2 {
    //                     //         item1 == item2
    //                     //     }
    //                     // }
    //                     FilterVarEquality::AllOverAllOver(all_over1, all_over2) => {
    //                         for item1 in all_over1.iter() {
    //                             for item2 in all_over2.iter() {
    //                                 if item1 != item2 {
    //                                     return Ok(PEResult::Reject);
    //                                 }
    //                             }
    //                         }
    //                         Ok(PEResult::Filter(query))
    //                     }

    //                     // every all_over_item in all_over {
    //                     //     some any_over_item in any_over {
    //                     //         all_over_item == any_over_item
    //                     //     }
    //                     // }
    //                     FilterVarEquality::AllOverAnyOver(all_over, any_over) => match any_over {
    //                         FilterVar::Unknown(_any_over_path) => todo!(),
    //                         FilterVar::Value(any_over) => {
    //                             match any_over {
    //                                 Value::Array(any_over_array) => {
    //                                     for all_over_item in all_over.iter() {
    //                                         let mut reject = true;
    //                                         for any_over_item in any_over_array {
    //                                             if any_over_item == all_over_item {
    //                                                 reject = false;
    //                                             }
    //                                         }
    //                                         if reject {
    //                                             return Ok(PEResult::Reject);
    //                                         }
    //                                     }
    //                                     Ok(PEResult::Filter(query))
    //                                 },
    //                                 _ => return Err(anyhow::Error::msg(format!("cannot iterate over non-iterable value: {any_over}"))),
    //                             }
    //                         },
    //                         _ => todo!(),
    //                     }

    //                     // NOTE: weird setup, basically can collapse to checking if all
    //                     // values are equal and then turn it into a FilterVarEquality::UnknownValue
    //                     //
    //                     // every item in all_over {
    //                     //     item == unknown
    //                     // }
    //                     FilterVarEquality::AllOverUnknown(all_over, unknown) => todo!(),

    //                     // every item in values {
    //                     //     item == value
    //                     // }
    //                     FilterVarEquality::AllOverValue(values, value) => {
    //                         for item in values.iter() {
    //                             if item != *value {
    //                                 return Ok(PEResult::Reject);
    //                             }
    //                         }
    //                         Ok(PEResult::Filter(query))
    //                     }

    //                     // some any_over_item in any_over {
    //                     //     every all_over_item in all_over {
    //                     //         any_over_item == all_over_item
    //                     //     }
    //                     // }
    //                     FilterVarEquality::AnyOverAllOver(any_over, all_over) => match any_over {
    //                         FilterVar::Unknown(_any_over_path) => todo!(),
    //                         FilterVar::Value(any_over) => match any_over {
    //                             Value::Array(any_over_array) => {
    //                                 let mut reject = true;
    //                                 for any_over_item in any_over_array {
    //                                     let mut all = true;
    //                                     for all_over_item in all_over.iter() {
    //                                         if any_over_item != all_over_item {
    //                                             all = false;
    //                                         }
    //                                     }
    //                                     if all {
    //                                         reject = false;
    //                                         break;
    //                                     }
    //                                 }
    //                                 if reject {
    //                                     return Ok(PEResult::Reject);
    //                                 }
    //                                 Ok(PEResult::Filter(query))
    //                             },
    //                             _ => return Err(anyhow::Error::msg(format!("cannot iterate over non-iterable value: {any_over}"))),
    //                         },
    //                         _ => todo!(),
    //                     }

    //                     // some item1 in any_over1 {
    //                     //     some item2 in any_over2 {
    //                     //         item1 == item2
    //                     //     }
    //                     // }
    //                     FilterVarEquality::AnyOverAnyOver(_filter_var1, _filter_var2) => todo!(),

    //                     // some item in any_over {
    //                     //     item == unknown
    //                     // }
    //                     FilterVarEquality::AnyOverUnknown(filter_var, _path) => match filter_var {
    //                         FilterVar::Unknown(_any_over_path) => todo!(),
    //                         FilterVar::Value(any_over_value) => match any_over_value {
    //                             Value::Array(_any_over_array) => todo!(),
    //                             value => return Err(anyhow::Error::msg(format!("Unauthorized: AnyOverUnknown: unable to iterate over non-array:\n{value}"))),
    //                         },
    //                         _ => todo!(),
    //                     }

    //                     // some item in any_over {
    //                     //     item == value
    //                     // }
    //                     FilterVarEquality::AnyOverValue(filter_var, value) => match filter_var {
    //                         FilterVar::Unknown(_any_over_path) => todo!(),
    //                         FilterVar::Value(any_over_value) => match any_over_value {
    //                             Value::Array(any_over_array) => {
    //                                 let mut reject = true;
    //                                 for any_over_item in any_over_array {
    //                                     if any_over_item == *value {
    //                                         reject = false;
    //                                     }
    //                                 }
    //                                 if reject {
    //                                     return Ok(PEResult::Reject);
    //                                 }
    //                                 Ok(PEResult::Filter(query))
    //                             },
    //                             value => return Err(anyhow::Error::msg(format!("Unauthorized: AnyOverUnknown: unable to iterate over non-array:\n{value}"))),
    //                         },
    //                         _ => todo!(),
    //                     }

    //                     // unknown == unknown
    //                     FilterVarEquality::UnknownUnknown(Unknown(_unknown1), Unknown(_unknown2)) => {
    //                         todo!()
    //                     },

    //                     // unknown == value
    //                     FilterVarEquality::UnknownValue(Unknown(unknown), value) => {
    //                         let [service, entity, field] = unknown
    //                             .try_into()
    //                             .map_err(|err| anyhow::Error::msg(format!("unable to cast unkonwn as (service, entity, field) tuple: {err}")))?;
    //                         let mut query = query;
    //                         let service = schemas.get(&**service);
    //                         let table = schema.table(&**entity);
    //                         if !join_set.contains(&**entity) {
    //                            query = query.join_target(table);
    //                         }
    //                         match value {
    //                             Value::Number(number) => {
    //                                 if number.is_i64() {
    //                                     let number = number.as_i64().unwrap();
    //                                     if let Ok(number) = i32::try_from(number) {
    //                                         let column = table.column::<Integer, _>(&**field);
    //                                         let query = query.filter(Box::new(column.eq(number)));
    //                                         Ok(PEResult::Filter(query))
    //                                     } else {
    //                                         let column = table.column::<BigInt, _>(&**field);
    //                                         let query = query.filter(Box::new(column.eq(number)));
    //                                         Ok(PEResult::Filter(query))
    //                                     }
    //                                 } else if number.is_u64() {
    //                                     let number = number.as_u64().unwrap();
    //                                     if let Ok(number) = i64::try_from(number) {
    //                                         let column = table.column::<BigInt, _>(&**field);
    //                                         let query = query.filter(Box::new(column.eq(number)));
    //                                         Ok(PEResult::Filter(query))
    //                                     } else {
    //                                         return Err(anyhow::Error::msg(format!("numeric value to large to filter on: {number}")));
    //                                     }
    //                                 } else {
    //                                     let number = number.as_f64().unwrap() as f32;
    //                                     let column = table.column::<Float, _>(&**field);
    //                                     let query = query.filter(Box::new(column.eq(number)));
    //                                     Ok(PEResult::Filter(query))
    //                                 }
    //                             },
    //                             Value::String(string) => {
    //                                 let column = table.column::<Text, _>(&**field);
    //                                 let query = query.filter(Box::new(column.eq(string)));
    //                                 Ok(PEResult::Filter(query))
    //                             },
    //                             _ => return Err(anyhow::Error::msg("unable to filter value")),
    //                         }
    //                     }

    //                     // value1 == value2
    //                     FilterVarEquality::ValueValue(value1, value2) => {
    //                         if value1 != value2 {
    //                             return Ok(PEResult::Reject);
    //                         }
    //                         Ok(PEResult::Filter(query))
    //                     }
    //                 }
    //             }
    //         }

    //         impl Evaluation {
    //             pub fn parse(&self, input: &PEQueryInput<'_>) -> Result<PEResult, anyhow::Error> {
    //                 let filters = match self {
    //                     Self::Filter(filters) => filters,
    //                     Self::Accept => return Ok(PEResult::Accept),
    //                 };

    //                 let mut join_set = JoinSet::default();
    //                 let mut pe_filter = None;

    //                 for filter in filters.iter() {
    //                     let new_pe_filter = match filter.get_query(&mut join_set)? {
    //                         PEResult::Filter(pe_filter) => pe_filter,
    //                         PEResult::Accept => return Ok(PEResult::Accept),
    //                         PEResult::Reject => return Ok(PEResult::Reject),
    //                     };
    //                     pe_filter = match (pe_filter, new_pe_filter) {
    //                         (None, new_pe_filter) => new_pe_filter,
    //                         (Some(pe_filter), None) => Some(pe_filter),
    //                         (Some(pe_filter), Some(new_pe_filter)) => PEFilter {
    //                             where_clause: pe_filter.where_clause.and(new_pe_filter.where_clause),
    //                             joins: pe_filter.
    //                         },
    //                     };
    //                     query = query.and(new_query);
    //                 }

    //                 Ok(PEResult::Filter(pe_filter))
    //             }
    //         }

    //         #[cfg(test)]
    //         mod tests {
    //             use super::*;
    //             use accounts_db::{_DbAccount, DbAccount, schema::DYNAMIC_SCHEMA};
    //             use diesel_util::Page;
    //             use tokio::test;

    //             #[test]
    //             async fn test_filter<D: Db>(db: &D) -> Result<(), anyhow::Error> {
    //                 let evaluation = Evaluation::Filter(vec![Filter::Equality {
    //                     negated: false,
    //                     left: FilterVar::Unknown(["account".into(), "id".into()]),
    //                     right: FilterVar::Value(json!("e3663d3c-8c2c-442f-9885-a7e2508e0760")),
    //                 }]);

    //                 let _db_accounts: Vec<_DbAccount> = evaluation.eval_filter(db, &DYNAMIC_SCHEMA, vec![Page::new(10, 0).unwrap()]).await?;
    //                 _db_accounts.into_iter().map::<DbAccount>(Into::into).collect();

    //                 Ok(())
    //             }
    //         }
    //     }
    // }

    #[derive(Clone, Debug)]
    struct Rule<'a: 'b, 'b> {
        positive: bool,
        operations: Vec<Operation<'a, 'b>>,
    }

    #[derive(Clone, Debug, Eq, PartialEq)]
    enum Operation<'a: 'b, 'b> {
        Assignment {
            key: &'b str,
            value: &'b OPAPolicyASTNode<'a>,
        },
        Boolean(bool),
        Equality {
            negated: bool,
            left: &'b OPAPolicyASTNode<'a>,
            right: &'b OPAPolicyASTNode<'a>,
        },
        Every {
            negated: bool,
            key: &'b str,
            value: &'b str,
            domain: &'b OPAPolicyASTNode<'a>,
            operations: Vec<Operation<'a, 'b>>,
        },
        Exists {
            negated: bool,
            key: &'b OPAPolicyASTNode<'a>,
        },
        In {
            negated: bool,
            key: &'b OPAPolicyASTNode<'a>,
            set: &'b OPAPolicyASTNode<'a>,
        },
    }

    impl Ord for Operation<'_, '_> {
        fn cmp(&self, rhs: &Self) -> Ordering {
            self.partial_cmp(rhs).unwrap()
        }
    }

    impl PartialOrd for Operation<'_, '_> {
        fn partial_cmp(&self, rhs: &Self) -> Option<Ordering> {
            match self {
                Self::Boolean(_) => Some(Ordering::Less),
                _ => match rhs {
                    Self::Boolean(_) => Some(Ordering::Greater),
                    _ => Some(Ordering::Equal),
                },
            }
        }
    }

    lazy_static! {
        static ref EXPECTED_NUM_OPERANDS: HashMap<&'static str, usize> =
            HashMap::from_iter([("eq", 2), ("internal.member_2", 2), ("neq", 2),].into_iter());
    }

    impl<'a: 'b, 'b> PEQueryResponseResultSupportRuleCondition<'a> {
        fn as_operation(&'b self) -> Result<Operation<'a, 'b>, anyhow::Error> {
            match &self.terms {
                PEQueryResponseResultSupportRuleConditionTerms::Scope(
                    PEQueryResponseResultSupportRuleConditionTermsScope {
                        domain,
                        key,
                        value,
                        conditions,
                    },
                ) => {
                    if domain.is_none() || key.is_none() || value.is_none() || conditions.is_none() {
                        return Err(anyhow::Error::msg(format!(
                            "cannot process scoped condition terms when either key/value/domain/body is none: {self:?}"
                        )));
                    }
                    let domain = domain.as_ref().unwrap();
                    let key = key.as_ref().unwrap();
                    let value = value.as_ref().unwrap();
                    let conditions = conditions.as_ref().unwrap();

                    let key = match key {
                        OPAPolicyASTNode::Var(key) => key,
                        _ => {
                            return Err(anyhow::Error::msg(format!(
                                "unexpected node type for scoped term's key: {self:?}"
                            )))
                        }
                    };
                    let value = match value {
                        OPAPolicyASTNode::Var(value) => value,
                        _ => {
                            return Err(anyhow::Error::msg(format!(
                                "unexpected node type for scoped term's value: {self:?}"
                            )))
                        }
                    };

                    Ok(Operation::Every {
                        operations: conditions.iter().map(Self::as_operation).collect::<Result<_, _>>()?,
                        negated: self.negated,
                        key,
                        value,
                        domain,
                    })
                }
                PEQueryResponseResultSupportRuleConditionTerms::Node(term) => match term {
                    OPAPolicyASTNode::Boolean(bool) => return Ok(Operation::Boolean(self.negated ^ bool)),
                    OPAPolicyASTNode::Ref(_) => Ok(Operation::Exists {
                        negated: self.negated,
                        key: term,
                    }),
                    _ => Err(anyhow::Error::msg(format!(
                        "unsupported single term type for condition: {self:?}"
                    ))),
                },
                PEQueryResponseResultSupportRuleConditionTerms::Nodes(terms) => {
                    if let OPAPolicyASTNode::Ref(node_ref) = &terms[0] {
                        let operation_name = format!("{node_ref}");
                        let expected_num_operands = EXPECTED_NUM_OPERANDS.get(&*operation_name).ok_or_else(|| {
                            anyhow::Error::msg(format!("unsupported condition operation `{operation_name}`: {self:?}"))
                        })?;
                        if terms.len() == expected_num_operands + 1 {
                            match &*operation_name {
                                "eq" => {
                                    if self.generated {
                                        if let OPAPolicyASTNode::Var(key) = &terms[1] {
                                            return Ok(Operation::Assignment { value: &terms[2], key });
                                        } else {
                                            return Err(anyhow::Error::msg(
                                                "expected generated term to be an assignment statement".to_string(),
                                            ));
                                        }
                                    }
                                    return Ok(Operation::Equality {
                                        left: &terms[1],
                                        right: &terms[2],
                                        negated: self.negated,
                                    });
                                }
                                "internal.member_2" => {
                                    return Ok(Operation::In {
                                        key: &terms[1],
                                        set: &terms[2],
                                        negated: self.negated,
                                    })
                                }
                                "neq" => {
                                    return Ok(Operation::Equality {
                                        left: &terms[1],
                                        right: &terms[2],
                                        negated: !self.negated,
                                    })
                                }
                                _ => {
                                    return Err(anyhow::Error::msg(format!(
                                        "unsupported condition operation `{operation_name}`: {self:?}"
                                    )))
                                }
                            }
                        } else {
                            return Err(anyhow::Error::msg(format!(
                                "unexpected number of operands for operation `{operation_name}`: {self:?}"
                            )));
                        }
                    }
                    if let OPAPolicyASTNode::Boolean(bool) = &terms[0] {
                        return Ok(Operation::Boolean(self.negated ^ bool));
                    }
                    Err(anyhow::Error::msg(format!(
                        "unexpected terms configuration for body: {self:?}"
                    )))
                }
            }
        }
    }

    #[derive(Clone, Debug, Eq, PartialEq)]
    enum Var<'a: 'b, 'b> {
        AllOver(Cow<'b, Vec<Value>>),
        AnyOver(Box<Var<'a, 'b>>),
        Unknown(OPAPolicyASTNodeRefRef<'a, 'b>),
        Value(Cow<'b, Value>),
    }

    impl Var<'_, '_> {
        fn is_unknown(&self) -> bool {
            matches!(self, Self::AnyOver(_) | Self::Unknown(_))
        }
    }

    #[derive(Clone, Debug, Eq, Hash, PartialEq)]
    enum VarKey<'a: 'b, 'b> {
        Ref(Cow<'b, OPAPolicyASTNodeRef<'a>>),
        String(&'b str),
    }

    type VarScope<'a, 'b> = HashMap<VarKey<'a, 'b>, Var<'a, 'b>>;

    #[derive(Clone, Debug, Eq, PartialEq)]
    enum _Evaluation {
        Accept,
        Filter(Vec<Filter>),
        Reject,
    }

    impl _Evaluation {
        fn and(self, rhs: Self) -> Self {
            match self {
                _Evaluation::Accept => rhs,
                _Evaluation::Filter(mut filters) => match rhs {
                    _Evaluation::Filter(mut rhs_filters) => {
                        rhs_filters.append(&mut filters);
                        _Evaluation::Filter(rhs_filters)
                    }
                    _Evaluation::Reject => _Evaluation::Filter(filters),
                    _Evaluation::Accept => _Evaluation::Accept,
                },
                _Evaluation::Reject => _Evaluation::Reject,
            }
        }

        fn or(self, rhs: Self) -> Self {
            match self {
                _Evaluation::Accept => _Evaluation::Accept,
                _Evaluation::Filter(mut filters) => match rhs {
                    _Evaluation::Filter(mut rhs_filters) => {
                        filters.append(&mut rhs_filters);
                        _Evaluation::Filter(filters)
                    }
                    _Evaluation::Reject => _Evaluation::Filter(filters),
                    _Evaluation::Accept => _Evaluation::Accept,
                },
                _Evaluation::Reject => rhs,
            }
        }
    }

    impl From<bool> for _Evaluation {
        fn from(bool: bool) -> Self {
            if bool {
                Self::Accept
            } else {
                Self::Reject
            }
        }
    }

    impl PartialEq<bool> for _Evaluation {
        fn eq(&self, rhs: &bool) -> bool {
            match self {
                Self::Accept => *rhs,
                Self::Reject => !(*rhs),
                _ => false,
            }
        }
    }

    #[derive(Clone, Debug, Eq, PartialEq)]
    pub enum LogicalOp {
        And,
        Or,
    }

    impl LogicalOp {
        fn default_evaluation(&self) -> _Evaluation {
            match self {
                Self::And => _Evaluation::Accept,
                Self::Or => _Evaluation::Reject,
            }
        }
    }

    impl FilterVar {
        fn from(var: &Var<'_, '_>, action: &PEQueryInputAction<'_>) -> Result<Self, anyhow::Error> {
            Ok(match var {
                Var::AllOver(values) => Self::AllOver(AllOver((*values).clone().into_owned())),
                Var::AnyOver(var) => Self::AnyOver(AnyOver(Box::new(Self::from(var, action)?))),
                Var::Value(value) => Self::Value((*value).clone().into_owned()),
                Var::Unknown(node_ref) => {
                    let mut path_nodes = node_ref.0.iter().map(ToString::to_string).collect::<Vec<_>>();

                    let (should_splice_in_object_service_and_entity, split_index) = {
                        if path_nodes.len() > 3 && ["input", "action", "object"] == path_nodes[0..3] {
                            (true, 3)
                        } else if path_nodes.len() > 1 && &path_nodes[0] == "data" {
                            (false, 1)
                        } else {
                            return Err(anyhow::Error::msg("Unauthorized: unable to process unknown: path does lives in neither data nor input.action.object".to_string()));
                        }
                    };

                    let nodes = if should_splice_in_object_service_and_entity {
                        std::iter::once(action.service().to_string())
                            .chain(std::iter::once(action.entity().to_string()))
                            .chain(path_nodes.split_off(split_index))
                            .collect::<Vec<_>>()
                    } else {
                        path_nodes.split_off(split_index)
                    };

                    Self::Unknown(Unknown(nodes))
                }
            })
        }
    }

    #[async_recursion]
    async fn eval_operations<'a: 'b, 'b>(
        operations: &Vec<Operation<'a, 'b>>,
        positive: bool,
        opa_client: &OPAClient,
        unknowns: &HashSet<&str>,
        action: &'b PEQueryInputAction<'_>,
        mut var_scope: VarScope<'a, 'b>,
    ) -> Result<_Evaluation, anyhow::Error> {
        if *crate::OPA_DEBUG {
            info!("EVAL");
            info!("{operations:#?}");
        }

        if operations.is_empty() {
            return Ok(positive.into());
        }

        let mut num_assertions = 0;
        let mut evaluation = LogicalOp::And.default_evaluation();

        for operation in operations {
            if *crate::OPA_DEBUG {
                info!("{var_scope:#?}");
            }
            match operation {
                Operation::Assignment { key, value } => {
                    let value = get_value_in_scope(opa_client, unknowns, &mut var_scope, value).await?;
                    var_scope.insert(VarKey::String(key), value.clone());
                }
                Operation::Boolean(_) => todo!(),
                Operation::Equality { negated, left, right } => {
                    if let OPAPolicyASTNode::Var(left) = left {
                        if &left[..7] == "__local" && !var_scope.contains_key(&VarKey::String(left)) {
                            if let OPAPolicyASTNode::Ref(node_ref) = right {
                                if let OPAPolicyPathNode::Var(right_end) = &node_ref.0[node_ref.len() - 1] {
                                    if &right_end[..7] == "__local"
                                        && !var_scope.contains_key(&VarKey::String(right_end))
                                    {
                                        // "SOME IN" ASSIGNMENT

                                        let right_sub_value = get_ref_value_in_scope(
                                            opa_client,
                                            unknowns,
                                            &mut var_scope,
                                            Cow::Owned(OPAPolicyASTNodeRef(node_ref.0[..node_ref.len() - 1].to_vec())),
                                        )
                                        .await?;

                                        var_scope.insert(VarKey::String(left), Var::AnyOver(Box::new(right_sub_value)));

                                        continue;
                                    }
                                }
                            }
                        }
                    }

                    num_assertions += 1;

                    let left_value = get_value_in_scope(opa_client, unknowns, &mut var_scope, left).await?;
                    let right_value = get_value_in_scope(opa_client, unknowns, &mut var_scope, right).await?;

                    if left_value.is_unknown() || right_value.is_unknown() {
                        if *crate::OPA_DEBUG {
                            info!("UNKNOWN: {left_value:?} OR {right_value:?}");
                        }
                        evaluation = evaluation.and(_Evaluation::Filter(vec![Filter::Equality {
                            negated: *negated,
                            left: FilterVar::from(&left_value, action)?,
                            right: FilterVar::from(&right_value, action)?,
                        }]));
                    } else if *negated ^ (left_value == right_value) {
                        evaluation = evaluation.and(_Evaluation::Accept);
                    } else {
                        if *crate::OPA_DEBUG {
                            info!("EVAL: REJECT (0)");
                            info!("{evaluation:#?}");
                        }
                        return Ok(false.into());
                    }
                }
                Operation::Exists { negated, key } => {
                    num_assertions += 1;
                    let exists = get_value_in_scope(opa_client, unknowns, &mut var_scope, key)
                        .await
                        .is_ok();
                    if (*negated && exists) || (!*negated && !exists) {
                        if *crate::OPA_DEBUG {
                            info!("EVAL: REJECT (1)");
                            info!("{evaluation:#?}");
                        }
                        return Ok(false.into());
                    }
                }
                Operation::Every {
                    negated,
                    value,
                    domain,
                    operations,
                    ..
                } => {
                    num_assertions += 1;
                    if operations.is_empty() {
                        if *crate::OPA_DEBUG {
                            info!("EVAL: REJECT (2)");
                            info!("{evaluation:#?}");
                        }
                        return Ok(false.into());
                    }
                    if *negated {
                        todo!();
                    }
                    let mut sub_var_scope = var_scope.clone();
                    let domain_value = get_value_in_scope(opa_client, unknowns, &mut sub_var_scope, domain).await?;
                    match domain_value {
                        Var::Value(domain_value) => match domain_value {
                            Cow::Owned(Value::Array(domain_values)) => {
                                sub_var_scope.insert(VarKey::String(value), Var::AllOver(Cow::Owned(domain_values)))
                            }
                            Cow::Borrowed(Value::Array(domain_values)) => {
                                sub_var_scope.insert(VarKey::String(value), Var::AllOver(Cow::Borrowed(domain_values)))
                            }
                            domain_value => {
                                return Err(anyhow::Error::msg(format!(
                                    "Unauthorized: unable to iterate over non-array:\n{domain_value}"
                                )))
                            }
                        },
                        domain_value => sub_var_scope.insert(VarKey::String(value), domain_value),
                    };

                    evaluation = evaluation
                        .and(eval_operations(operations, positive, opa_client, unknowns, action, sub_var_scope).await?);
                }
                Operation::In { negated, key, set } => {
                    num_assertions += 1;
                    let key_value = get_value_in_scope(opa_client, unknowns, &mut var_scope, key).await?;
                    let set_value = get_value_in_scope(opa_client, unknowns, &mut var_scope, set).await?;

                    if key_value.is_unknown() || set_value.is_unknown() {
                        evaluation = evaluation.and(_Evaluation::Filter(vec![Filter::In {
                            negated: *negated,
                            key: FilterVar::from(&key_value, action)?,
                            set: FilterVar::from(&set_value, action)?,
                        }]));
                        continue;
                    }

                    let key_value = if let Var::Value(key_value) = key_value {
                        key_value
                    } else {
                        if *crate::OPA_DEBUG {
                            info!("{evaluation:#?}");
                        }
                        return Err(anyhow::Error::msg(format!(
                            "unable to test set membership for non-value key: {key_value:?}"
                        )));
                    };
                    let set_value = if let Var::Value(set_value) = set_value {
                        set_value
                    } else {
                        if *crate::OPA_DEBUG {
                            info!("{evaluation:#?}");
                        }
                        return Err(anyhow::Error::msg(format!(
                            "unable to test set membership for non-value set: {set_value:?}"
                        )));
                    };

                    match set_value.as_ref() {
                        Value::Array(array) => {
                            for item in array {
                                if *negated ^ (key_value.as_ref() == item) {
                                    if *crate::OPA_DEBUG {
                                        info!("EVAL: REJECT (3)");
                                        info!("{evaluation:#?}");
                                    }
                                    return Ok(false.into());
                                }
                            }
                        }
                        Value::Object(object) => {
                            for key in object.keys() {
                                // TODO: confirm whether key membership or value membership is how this should proceed
                                if *negated ^ (key_value.as_ref() == key) {
                                    if *crate::OPA_DEBUG {
                                        info!("EVAL: REJECT (4)");
                                        info!("{evaluation:#?}");
                                    }
                                    return Ok(false.into());
                                }
                            }
                        }
                        _ => {
                            if *crate::OPA_DEBUG {
                                info!("{evaluation:#?}");
                            }
                            return Err(anyhow::Error::msg(format!(
                                "unable to test set membership for non array/object set: {set_value:?}"
                            )));
                        }
                    };
                }
            }
        }

        if num_assertions == 0 {
            if *crate::OPA_DEBUG {
                info!("EVAL: REJECT (5)");
                info!("{evaluation:#?}");
            }
            return Ok(positive.into());
        }

        if *crate::OPA_DEBUG {
            info!("{evaluation:#?}");
        }
        Ok(evaluation)
    }

    async fn get_value_in_scope<'a: 'b, 'b>(
        opa_client: &OPAClient,
        unknowns: &HashSet<&str>,
        var_scope: &mut VarScope<'a, 'b>,
        node: &'b OPAPolicyASTNode<'a>,
    ) -> Result<Var<'a, 'b>, anyhow::Error> {
        match node {
            OPAPolicyASTNode::Array(_)
            | OPAPolicyASTNode::Boolean(_)
            | OPAPolicyASTNode::Number(_)
            | OPAPolicyASTNode::String(_) => Ok(Var::Value(Cow::Owned(node.try_into()?))),
            OPAPolicyASTNode::Call(_) => todo!(),
            OPAPolicyASTNode::Ref(node_ref) => {
                get_ref_value_in_scope(opa_client, unknowns, var_scope, Cow::Borrowed(node_ref)).await
            }
            OPAPolicyASTNode::Set(_) => todo!(),
            OPAPolicyASTNode::Var(var) => var_scope
                .get(&VarKey::String(var))
                .map(Clone::clone)
                .ok_or_else(|| anyhow::Error::msg(format!("no variable found in scope with name `{var}`"))),
        }
    }

    async fn get_ref_value_in_scope<'a: 'b, 'b>(
        _opa_client: &OPAClient,
        unknowns: &HashSet<&str>,
        var_scope: &mut VarScope<'a, 'b>,
        node_ref: Cow<'b, OPAPolicyASTNodeRef<'a>>,
    ) -> Result<Var<'a, 'b>, anyhow::Error> {
        let var_key = VarKey::Ref(node_ref);

        if let Some(value) = var_scope.get(&var_key) {
            return Ok(value.clone());
        }
        let node_ref = match var_key {
            VarKey::Ref(node_ref) => node_ref,
            _ => unreachable!(),
        };

        if unknowns.contains(&*format!("{node_ref}")) {
            let var = Var::Unknown(node_ref.clone().into());
            var_scope.insert(VarKey::Ref(node_ref), var.clone());
            return Ok(var);
        }

        if node_ref.as_ref().0[0] == "data" {
            todo!();
        }

        Err(anyhow::Error::msg(format!("invalid ref node `{node_ref}`")))
    }

    fn add_to_scope<'a: 'b, 'b>(
        var_scope: &mut VarScope<'a, 'b>,
        value: &'a Value,
        prefix: &OPAPolicyASTNodeRef<'a>,
        key: OPAPolicyPathNode<'a>,
    ) {
        if let Value::Null = value {
            return;
        }

        let mut new_prefix = prefix.clone();
        new_prefix.0.push(key);

        match value {
            Value::Array(array) => {
                for (i, item) in array.iter().enumerate() {
                    add_to_scope(var_scope, item, &new_prefix, OPAPolicyPathNode::Number(i));
                }
            }
            Value::Object(object) => {
                for (key, item) in object.into_iter() {
                    add_to_scope(var_scope, item, &new_prefix, OPAPolicyPathNode::String(key));
                }
            }
            _ => {}
        };
        var_scope.insert(VarKey::Ref(Cow::Owned(new_prefix)), Var::Value(Cow::Borrowed(value)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::test;

    #[test]
    async fn filter_roles() -> Result<(), anyhow::Error> {
        let opa_client = OPAClient::test()?;

        let evaluation = pe_query(
            &opa_client,
            PEQuery::builder()
                .input(
                    PEQueryInput::builder()
                        .token(Some("eyJ0eXAiOiJKV1QiLCJhbGciOiJSUzUxMiJ9.eyJleHAiOjE2NjU0MjY0NjYsImlhdCI6MTY2NDgyMTY2NiwiaXNzIjoiYWNjb3VudHMiLCJzdGF0ZSI6eyJhY2NvdW50X2lkIjoiZTM2NjNkM2MtOGMyYy00NDJmLTk4ODUtYTdlMjUwOGUwNzYwIiwicm9sZV9pZHMiOlsiM2Q5NzI0MjUtYjNjOS00ZGVkLThjNjYtNDA0NDM0ZWM0NzczIl19LCJzdWIiOiJlMzY2M2QzYy04YzJjLTQ0MmYtOTg4NS1hN2UyNTA4ZTA3NjAifQ.QF4GKCc6eq9USHklr1Aakza1YegvNFkOWxn6MIsCeMSC4O9c2aVumUiJ9qANbu_OeHWDR9d0VKB9YfKc-m_5bz_jkFrB-8qlAzQiUYhfBwT28a2iswK-8lLqLPUjwlYz9QgwRNfMXMcpBhGp2hteGqFoDPxLIzGxgUAgBNuR8nS27P4iFEDD6axMaW4ET_JAXa2Hh0lPyPHRi2bnbOlC7XbQv6Ir9rZgLk1z2ddqPKKv1zmZd0UUhXkJaCfSiXO3qK2tnzJMZ1e6zv1_oxYY08iG3Ux16yHvlxHOYmsBqIGEXngWBgfwV3EMUK6q97tEW9y_WWPexRHe8Rd9NgWdVb9RzvHSCRKIFDfkyjkLsOqJuoJQC1Zw0KhuMoDXguKCOxtS-yiDLDwAKjHusGSI9Bt130ZhGLkDdP_YEBmyhZ2h2noTwTTFIYL_tRcXVVMwMrmaaKd7VR4SXO9d9caXp0wvdeVgwZSYi3-8xEDRvH3L_AAPVhQ73LxraI0ua4V5"))
                        .action(PEQueryInputAction::Read {
                            service: Cow::Borrowed("accounts"),
                            entity: Cow::Borrowed("role"),
                        })
                        .build(),
                )
                .build(),
            json!({
                "app": {
                    "subject": {
                        "account_id": "e3663d3c-8c2c-442f-9885-a7e2508e0760",
                        "role_ids": ["3d972425-b3c9-4ded-8c66-404434ec4773"],
                    },
                },
            }),
        )
            .await?;

        assert_eq!(
            Evaluation::Filter(vec![Filter::Equality {
                negated: false,
                left: FilterVar::Unknown(Unknown(vec!["accounts".into(), "role".into(), "id".into()])),
                right: FilterVar::AnyOver(AnyOver(Box::new(FilterVar::Value(json!([
                    "3d972425-b3c9-4ded-8c66-404434ec4773"
                ]))))),
            },]),
            evaluation,
        );

        Ok(())
    }

    #[test]
    async fn filter_accounts() -> Result<(), anyhow::Error> {
        let opa_client = OPAClient::test()?;

        let evaluation = pe_query(
            &opa_client,
            PEQuery::builder()
                .input(
                    PEQueryInput::builder()
                        .token(Some("eyJ0eXAiOiJKV1QiLCJhbGciOiJSUzUxMiJ9.eyJleHAiOjE2NjU0MjY0NjYsImlhdCI6MTY2NDgyMTY2NiwiaXNzIjoiYWNjb3VudHMiLCJzdGF0ZSI6eyJhY2NvdW50X2lkIjoiZTM2NjNkM2MtOGMyYy00NDJmLTk4ODUtYTdlMjUwOGUwNzYwIiwicm9sZV9pZHMiOlsiM2Q5NzI0MjUtYjNjOS00ZGVkLThjNjYtNDA0NDM0ZWM0NzczIl19LCJzdWIiOiJlMzY2M2QzYy04YzJjLTQ0MmYtOTg4NS1hN2UyNTA4ZTA3NjAifQ.QF4GKCc6eq9USHklr1Aakza1YegvNFkOWxn6MIsCeMSC4O9c2aVumUiJ9qANbu_OeHWDR9d0VKB9YfKc-m_5bz_jkFrB-8qlAzQiUYhfBwT28a2iswK-8lLqLPUjwlYz9QgwRNfMXMcpBhGp2hteGqFoDPxLIzGxgUAgBNuR8nS27P4iFEDD6axMaW4ET_JAXa2Hh0lPyPHRi2bnbOlC7XbQv6Ir9rZgLk1z2ddqPKKv1zmZd0UUhXkJaCfSiXO3qK2tnzJMZ1e6zv1_oxYY08iG3Ux16yHvlxHOYmsBqIGEXngWBgfwV3EMUK6q97tEW9y_WWPexRHe8Rd9NgWdVb9RzvHSCRKIFDfkyjkLsOqJuoJQC1Zw0KhuMoDXguKCOxtS-yiDLDwAKjHusGSI9Bt130ZhGLkDdP_YEBmyhZ2h2noTwTTFIYL_tRcXVVMwMrmaaKd7VR4SXO9d9caXp0wvdeVgwZSYi3-8xEDRvH3L_AAPVhQ73LxraI0ua4V5"))
                        .action(PEQueryInputAction::Read {
                            service: Cow::Borrowed("accounts"),
                            entity: Cow::Borrowed("account"),
                        })
                        .build(),
                )
                .build(),
            json!({
                "app": {
                    "subject": {
                        "account_id": "e3663d3c-8c2c-442f-9885-a7e2508e0760",
                        "role_ids": ["3d972425-b3c9-4ded-8c66-404434ec4773"],
                    },
                },
            }),
        )
            .await?;

        assert_eq!(
            Evaluation::Filter(vec![Filter::Equality {
                negated: false,
                left: FilterVar::Unknown(Unknown(vec!["accounts".into(), "account".into(), "id".into()])),
                right: FilterVar::Value(json!("e3663d3c-8c2c-442f-9885-a7e2508e0760")),
            },]),
            evaluation,
        );

        Ok(())
    }

    #[test]
    async fn filter_role_attachments() -> Result<(), anyhow::Error> {
        let opa_client = OPAClient::test()?;

        let evaluation_is_err = pe_query(
            &opa_client,
            PEQuery::builder()
                .input(
                    PEQueryInput::builder()
                        .token(Some("eyJ0eXAiOiJKV1QiLCJhbGciOiJSUzUxMiJ9.eyJleHAiOjE2NjU0MjY0NjYsImlhdCI6MTY2NDgyMTY2NiwiaXNzIjoiYWNjb3VudHMiLCJzdGF0ZSI6eyJhY2NvdW50X2lkIjoiZTM2NjNkM2MtOGMyYy00NDJmLTk4ODUtYTdlMjUwOGUwNzYwIiwicm9sZV9pZHMiOlsiM2Q5NzI0MjUtYjNjOS00ZGVkLThjNjYtNDA0NDM0ZWM0NzczIl19LCJzdWIiOiJlMzY2M2QzYy04YzJjLTQ0MmYtOTg4NS1hN2UyNTA4ZTA3NjAifQ.QF4GKCc6eq9USHklr1Aakza1YegvNFkOWxn6MIsCeMSC4O9c2aVumUiJ9qANbu_OeHWDR9d0VKB9YfKc-m_5bz_jkFrB-8qlAzQiUYhfBwT28a2iswK-8lLqLPUjwlYz9QgwRNfMXMcpBhGp2hteGqFoDPxLIzGxgUAgBNuR8nS27P4iFEDD6axMaW4ET_JAXa2Hh0lPyPHRi2bnbOlC7XbQv6Ir9rZgLk1z2ddqPKKv1zmZd0UUhXkJaCfSiXO3qK2tnzJMZ1e6zv1_oxYY08iG3Ux16yHvlxHOYmsBqIGEXngWBgfwV3EMUK6q97tEW9y_WWPexRHe8Rd9NgWdVb9RzvHSCRKIFDfkyjkLsOqJuoJQC1Zw0KhuMoDXguKCOxtS-yiDLDwAKjHusGSI9Bt130ZhGLkDdP_YEBmyhZ2h2noTwTTFIYL_tRcXVVMwMrmaaKd7VR4SXO9d9caXp0wvdeVgwZSYi3-8xEDRvH3L_AAPVhQ73LxraI0ua4V5"))
                        .action(PEQueryInputAction::Read {
                            service: Cow::Borrowed("accounts"),
                            entity: Cow::Borrowed("role_attachment"),
                        })
                        .build(),
                )
                .build(),
            json!({
                "app": {
                    "subject": {
                        "account_id": "e3663d3c-8c2c-442f-9885-a7e2508e0760",
                        "role_ids": ["3d972425-b3c9-4ded-8c66-404434ec4773"],
                    },
                },
            }),
        )
            .await
            .is_err();

        assert!(evaluation_is_err);

        Ok(())
    }

    #[test]
    async fn filter_role_inheritances() -> Result<(), anyhow::Error> {
        let opa_client = OPAClient::test()?;

        let evaluation_is_err = pe_query(
            &opa_client,
            PEQuery::builder()
                .input(
                    PEQueryInput::builder()
                        .token(Some("eyJ0eXAiOiJKV1QiLCJhbGciOiJSUzUxMiJ9.eyJleHAiOjE2NjU0MjY0NjYsImlhdCI6MTY2NDgyMTY2NiwiaXNzIjoiYWNjb3VudHMiLCJzdGF0ZSI6eyJhY2NvdW50X2lkIjoiZTM2NjNkM2MtOGMyYy00NDJmLTk4ODUtYTdlMjUwOGUwNzYwIiwicm9sZV9pZHMiOlsiM2Q5NzI0MjUtYjNjOS00ZGVkLThjNjYtNDA0NDM0ZWM0NzczIl19LCJzdWIiOiJlMzY2M2QzYy04YzJjLTQ0MmYtOTg4NS1hN2UyNTA4ZTA3NjAifQ.QF4GKCc6eq9USHklr1Aakza1YegvNFkOWxn6MIsCeMSC4O9c2aVumUiJ9qANbu_OeHWDR9d0VKB9YfKc-m_5bz_jkFrB-8qlAzQiUYhfBwT28a2iswK-8lLqLPUjwlYz9QgwRNfMXMcpBhGp2hteGqFoDPxLIzGxgUAgBNuR8nS27P4iFEDD6axMaW4ET_JAXa2Hh0lPyPHRi2bnbOlC7XbQv6Ir9rZgLk1z2ddqPKKv1zmZd0UUhXkJaCfSiXO3qK2tnzJMZ1e6zv1_oxYY08iG3Ux16yHvlxHOYmsBqIGEXngWBgfwV3EMUK6q97tEW9y_WWPexRHe8Rd9NgWdVb9RzvHSCRKIFDfkyjkLsOqJuoJQC1Zw0KhuMoDXguKCOxtS-yiDLDwAKjHusGSI9Bt130ZhGLkDdP_YEBmyhZ2h2noTwTTFIYL_tRcXVVMwMrmaaKd7VR4SXO9d9caXp0wvdeVgwZSYi3-8xEDRvH3L_AAPVhQ73LxraI0ua4V5"))
                        .action(PEQueryInputAction::Read {
                            service: Cow::Borrowed("accounts"),
                            entity: Cow::Borrowed("role_inheritance"),
                        })
                        .build(),
                )
                .build(),
            json!({
                "app": {
                    "subject": {
                        "account_id": "e3663d3c-8c2c-442f-9885-a7e2508e0760",
                        "role_ids": ["3d972425-b3c9-4ded-8c66-404434ec4773"],
                    },
                },
            }),
        )
            .await
            .is_err();

        assert!(evaluation_is_err);

        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize)]
struct PEQueryResponse<'a> {
    #[serde(borrow)]
    result: PEQueryResponseResult<'a>,
}

#[derive(Clone, Debug, Deserialize)]
struct PEQueryResponseResult<'a> {
    #[serde(borrow)]
    queries: Option<Vec<PEQueryResponseResultQuery<'a>>>,
    #[serde(borrow, rename = "support")]
    supports: Option<Vec<PEQueryResponseResultSupport<'a>>>,
}

#[derive(Clone, Debug, Deserialize)]
struct PEQueryResponseResultQuery<'a>(#[serde(borrow)] pub Vec<PEQueryResponseResultQueryNode<'a>>);

#[derive(Clone, Debug, Deserialize)]
struct PEQueryResponseResultQueryNode<'a> {
    #[serde(borrow, deserialize_with = "OPAPolicyASTNode::deserialize_terms")]
    terms: Vec<OPAPolicyASTNode<'a>>,
}

#[derive(Clone, Debug, Deserialize)]
struct PEQueryResponseResultSupport<'a> {
    #[serde(borrow)]
    package: PEQueryResponseResultSupportPackage<'a>,
    #[serde(borrow)]
    rules: Vec<PEQueryResponseResultSupportRule<'a>>,
}

#[derive(Clone, Deserialize)]
struct PEQueryResponseResultSupportPackage<'a> {
    #[serde(borrow, rename = "path")]
    paths: Vec<OPAPolicyPathNode<'a>>,
}

impl Debug for PEQueryResponseResultSupportPackage<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let paths = self.paths.iter().map(|path| path.to_string()).collect::<Vec<_>>();
        write!(f, r#""{}""#, paths.join("."))
    }
}

#[derive(Clone, Debug, Deserialize)]
struct PEQueryResponseResultSupportRule<'a> {
    #[serde(borrow)]
    head: PEQueryResponseResultSupportRuleHead<'a>,
    #[serde(borrow, rename = "body")]
    conditions: Vec<PEQueryResponseResultSupportRuleCondition<'a>>,
}

#[derive(Clone, Debug, Deserialize)]
struct _PEQueryResponseResultSupportRuleHead<'a> {
    name: &'a str,
    #[serde(borrow)]
    key: Option<OPAPolicyASTNode<'a>>,
    #[serde(borrow)]
    value: Option<OPAPolicyASTNode<'a>>,
}

#[derive(Clone, Debug)]
struct PEQueryResponseResultSupportRuleHead<'a> {
    name: &'a str,
    kind: PEQueryResponseResultSupportRuleHeadKind<'a>,
}

#[derive(Clone, Debug)]
enum PEQueryResponseResultSupportRuleHeadKind<'a> {
    Key(OPAPolicyASTNode<'a>),
    Value(OPAPolicyASTNode<'a>),
}

impl<'de: 'a, 'a> Deserialize<'de> for PEQueryResponseResultSupportRuleHead<'a> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let head = _PEQueryResponseResultSupportRuleHead::deserialize(deserializer)?;
        if let Some(key) = head.key {
            return Ok(Self {
                name: head.name,
                kind: PEQueryResponseResultSupportRuleHeadKind::Key(key),
            });
        }
        if let Some(value) = head.value {
            return Ok(Self {
                name: head.name,
                kind: PEQueryResponseResultSupportRuleHeadKind::Value(value),
            });
        }
        Err(D::Error::custom(
            "PEQueryResponseResultSupportRuleHead expected either a `key` or `value` field, update data model if this error is output",
        ))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
struct PEQueryResponseResultSupportRuleCondition<'a> {
    index: usize,
    #[serde(default)]
    negated: bool,
    #[serde(default)]
    generated: bool,
    #[serde(borrow)]
    terms: PEQueryResponseResultSupportRuleConditionTerms<'a>,
}

impl Ord for PEQueryResponseResultSupportRuleCondition<'_> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.index.cmp(&other.index)
    }
}

impl PartialOrd for PEQueryResponseResultSupportRuleCondition<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.index.partial_cmp(&other.index)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(untagged)]
enum PEQueryResponseResultSupportRuleConditionTerms<'a> {
    Node(#[serde(borrow)] OPAPolicyASTNode<'a>),
    Nodes(#[serde(borrow)] Vec<OPAPolicyASTNode<'a>>),
    Scope(#[serde(borrow)] PEQueryResponseResultSupportRuleConditionTermsScope<'a>),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
struct PEQueryResponseResultSupportRuleConditionTermsScope<'a> {
    #[serde(borrow)]
    domain: Option<OPAPolicyASTNode<'a>>,
    #[serde(borrow)]
    key: Option<OPAPolicyASTNode<'a>>,
    #[serde(borrow)]
    value: Option<OPAPolicyASTNode<'a>>,
    #[serde(borrow, rename = "body")]
    conditions: Option<Vec<PEQueryResponseResultSupportRuleCondition<'a>>>,
}
