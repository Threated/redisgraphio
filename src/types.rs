use redis::{FromRedisValue, Value, RedisResult, from_redis_value};

use crate::{GraphValue, helpers::{create_rediserror, apply_macro}, FromGraphValue, from_graph_value};

/// ## Overview
/// Response type from redis graph
/// This type is generic over the a type that should represent the return value of each match in the query
/// For example a query like:
/// MATCH (a\:User {name\: 'You'}) RETURN a
/// Will return a Vec of whatever stands in the return clause
/// but the generic type in this case should not be `Node` it should be `(Node,)`
/// as there can be multiple comma seperated items returned by a single matched pattern.
#[derive(Debug)]
pub struct GraphResponse<T = GraphValue> where T: FromGraphValue {
    /// List of return type names e.g. RETURN item, otheritem
    /// will result in vec!["item", "otheritem"]
    /// This info is largely useless
    pub header: Vec<String>,
    /// Every item of data represents one match of a request
    /// each T is than the type returned by the return clause which is always a tuple-
    /// Even for RETURN 1 should be (i32, ) not i32 as there can always be multiple items
    /// returned from a single return clause
    pub data: Vec<T>,
    /// Statistics of the query e.g. "Cached execution: 1" or "Query internal execution time: 0.01337 milliseconds"
    pub statistics: Vec<String>
}

impl<T: FromGraphValue> GraphResponse<T> {
    fn parse_header(header: Vec<Value>) -> Vec<String> {
        // Somehow it is not possible to let redis convert the header to Vec<(i64, String)> on its own,
        // because it internally calls from_redis_values for tuple which has a weird internal implementation
        // which chunks the array and tries to collect every chunk as a (i64, String) Tuple when it should really just collect each item as a Tuple
        // This is probably only on --compact requests
        header
            .into_iter()
            .map(|v|
                from_redis_value::<(i64, String)>(&v).unwrap().1
            ).collect()
    }

    /// Parses a `redis::Value` into a `RedisResult<GraphResponse<T>>`
    pub fn parse_response(value: &Value) -> RedisResult<GraphResponse<T>> {
        match value {
            Value::Bulk(ref values) => {
                match values.len() {
                    1 => Ok(GraphResponse {
                        header: vec![],
                        data: vec![],
                        statistics: from_redis_value(&values[0])?
                    }),
                    3 => {
                        let (header, temp, statistics): (Vec<Value>, Vec<Vec<GraphValue>>, Vec<String>) = from_redis_value(value)?;
                        
                        Ok(GraphResponse {
                            header: <GraphResponse>::parse_header(header),
                            data: temp.into_iter().map(|arr|
                                from_graph_value(GraphValue::Array(arr))
                              ).collect::<RedisResult<_>>()?,
                            statistics
                        })
                    },
                    len => Err(create_rediserror(&format!("Can't parse response of length {} to GraphResponse", len)))
                }
            },
            _ => Err(create_rediserror("Invalid Response from Redis"))
        }
    }

    /// Try to get the value of the requested statistic
    pub fn get_statistic(&self, stat: GraphStatistic) -> Option<f64> {
        let start = stat.match_name();
        for str in self.statistics.iter() {
            if str.starts_with(start) {
                let (_, val) = str.split_once(": ")?;
                let val = val.split_once(' ').map_or(val, |x| x.0);
                return Some(val.parse().unwrap())
            }
        }
        None
    }
}


impl<T: FromGraphValue> FromRedisValue for GraphResponse<T> {
    fn from_redis_value(v: &Value) -> RedisResult<GraphResponse<T>> {
        GraphResponse::parse_response(v)
    }
}

/// Execution statistics
pub enum GraphStatistic {
    /// Number of labels added
    LabelsAdded,
    /// Number of nodes created
    NodesCreated,
    /// Number of relationships created
    RelationshipsCreated,
    /// Number of indices created
    IndicesCreated,
    /// Number of properties set
    PropertiesSet,
    /// Number of nodes deleted
    NodesDeleted,
    /// Number of relationships deleted
    RelationshipsDeleted,
    /// Number of indices deleted
    IndicesDeleted,
    /// Whether the query was cached
    CachedExecution,
    /// Internal execution time of the redis server
    ExecutionTime,
}

impl GraphStatistic {
    #[inline]
    pub(crate) const fn match_name(&self) -> &'static str{
        match self {
            GraphStatistic::LabelsAdded => "Labels added",
            GraphStatistic::NodesCreated => "Nodes created",
            GraphStatistic::RelationshipsCreated => "Relationships created",
            GraphStatistic::IndicesCreated => "Indices created",
            GraphStatistic::PropertiesSet => "Properties set",
            GraphStatistic::NodesDeleted => "Nodes deleted",
            GraphStatistic::RelationshipsDeleted => "Relationships deleted",
            GraphStatistic::IndicesDeleted => "Indices deleted",
            GraphStatistic::CachedExecution => "Cached execution",
            GraphStatistic::ExecutionTime =>  "Query internal",
        }
    }
}

/// Contains information for constructing the query.
/// Primarily generated by the [`query`] macro
#[derive(PartialEq, Debug)]
pub struct GraphQuery {
    /// The static query string
    pub query: &'static str,
    /// The dynamic Parameters to the query
    pub params: Vec<(&'static str, Parameter)>,
    /// Whether or not the request should be read only
    pub read_only: bool,
}

impl GraphQuery {
    pub(crate) fn read_type(&self) -> &'static str {
        if self.read_only {
            "GRAPH.RO_QUERY"
        } else {
            "GRAPH.QUERY"
        }
    }

    pub(crate) fn construct_query(&self) -> String {
        self.parse_params() + &self.query
    }

    fn parse_params(&self) -> String {
        if self.params.is_empty() {
            return String::new();
        }
        let mut prepend = String::from("CYPHER ");
        self.params.iter().for_each(|(key, value)| {
            prepend.push_str(&match value {
                Parameter::Int(int) => format!("{}={} ", key, int),
                Parameter::Double(double) => format!("{}={} ", key, double),
                Parameter::String(string) => format!(r#"{}="{}" "#, key, string.escape_default()),
            });
        });
        prepend
    }

    /// Adds a Parameter to the Parameter list that is escaped in the query 
    pub fn add_parameter<T: Into<Parameter>>(&mut self, key: &'static str, value: T) -> &mut GraphQuery {
        self.params.push((key, value.into()));
        self
    }

    /// Set read only
    pub fn read_only(&mut self, read_only: bool) -> &mut GraphQuery {
        self.read_only = read_only;
        self
    }
}

impl From<&'static str> for GraphQuery {
    fn from(query: &'static str) -> Self {
        GraphQuery { query, params: vec![], read_only: false}
    }
}

/// Used for inserting user data in the query and escaping it properly
/// This type gets primarilly constructed by the `query!` macro but can
/// also be constructed with `Parameter::from`
#[derive(Clone, PartialEq, Debug)]
pub enum Parameter {
    /// The Parameter is a String
    String(String),
    /// The Parameter is an Integer
    Int(i64),
    /// The Parameter is a Double
    Double(f64),
}

/// Macro for implementing the From Trait for a numeric type
macro_rules! parameter_from_int {
    ( $t:ty ) => {
        impl From<$t> for Parameter {
            fn from(id: $t) -> Self {
                Parameter::Int(i64::from(id))
            }
        }
    };
}

macro_rules! parameter_from_double {
    ( $t:ty ) => {
        impl From<$t> for Parameter {
            fn from(id: $t) -> Self {
                Parameter::Double(f64::from(id))
            }
        }
    };
}

apply_macro!(parameter_from_int, i8, i16, i32, i64, u8, u16, u32);
apply_macro!(parameter_from_double, f32, f64);

impl<'a> From<&'a str> for Parameter {
    fn from(string: &'a str) -> Self {
        Parameter::String(string.to_string())
    }
}

impl From<String> for Parameter {
    fn from(string: String) -> Self {
        Parameter::String(string)
    }
}

impl From<&String> for Parameter {
    fn from(string: &String) -> Self {
        Parameter::String(string.to_string())
    }
}
