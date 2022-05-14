use crate::{parse::GraphValue, Parameter};
use redis::{cmd, ConnectionLike, RedisResult};

pub trait GraphCommands: ConnectionLike + Sized {
    fn graph_query<Q: Into<GraphQuery>>(
        &mut self,
        graph: &str,
        query: Q,
    ) -> RedisResult<GraphValue> {
        let query = query.into();
        cmd(query.read_type())
            .arg(graph)
            .arg(query.construct_query())
            .arg("--compact")
            .query(self)
    }
}

impl<T> GraphCommands for T where T: ConnectionLike {}


pub struct GraphQuery {
    query: &'static str,
    params: Vec<(&'static str, Parameter)>,
    read_only: bool,
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
        self.parse_params() + self.query
    }

    fn parse_params(&self) -> String {
        if self.params.is_empty() {
            return String::new();
        }
        let mut prepend = String::from("CYPHER ");
        for (key, value) in self.params.iter() {
            prepend.push_str(&match value {
                Parameter::Int(int) => format!("{}={} ", key, int),
                Parameter::Double(double) => format!("{}={} ", key, double),
                Parameter::String(string) => format!(r#"{}="{}" "#, key, string),
            });
        }
        prepend
    }

    pub fn add_parameter<T: Into<Parameter>>(&mut self, key: &'static str, value: T) {
        self.params.push((key, value.into()))
    }
}

impl From<&'static str> for GraphQuery {
    fn from(query: &'static str) -> Self {
        GraphQuery { query, params: vec![], read_only: false}
    }
}

#[macro_export]
macro_rules! query {
    ( $s:expr ) => {{
        GraphQuery::from($s)
    }};
    ( $s:expr, { $( $k:expr => $v:expr ),+ } ) => {{
        GraphQuery {
            query: $s, read_only: false, vec![$(
                ($k, Parameter::from($v))
            )*]
        }
    }}
}
