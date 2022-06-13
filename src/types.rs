use redis::{FromRedisValue, Value, RedisResult, from_redis_value};

use crate::{GraphValue, helpers::{create_rediserror, apply_macro}, FromGraphValue, from_graph_value};

#[derive(Debug)]
pub struct GraphResponse<T = GraphValue> where T: FromGraphValue {
    pub header: Vec<String>,
    pub data: Vec<T>,
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
}


impl<T: FromGraphValue> FromRedisValue for GraphResponse<T> {
    fn from_redis_value(v: &Value) -> RedisResult<GraphResponse<T>> {
        GraphResponse::parse_response(v)
    }
}

#[derive(PartialEq, Debug)]
pub struct GraphQuery {
    pub query: &'static str,
    pub params: Vec<(&'static str, Parameter)>,
    pub read_only: bool,
}

impl GraphQuery {
    pub fn read_type(&self) -> &'static str {
        if self.read_only {
            "GRAPH.RO_QUERY"
        } else {
            "GRAPH.QUERY"
        }
    }

    pub fn construct_query(&self) -> String {
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

    pub fn add_parameter<T: Into<Parameter>>(&mut self, key: &'static str, value: T) -> &mut GraphQuery {
        self.params.push((key, value.into()));
        self
    }

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

#[derive(Clone, PartialEq, Debug)]
pub enum Parameter {
    String(String),
    Int(i64),
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
