use redis::{RedisResult, ErrorKind, RedisError};

use crate::{FromGraphValue, GraphValue};


/// So you dont have to write FromGraphValue::from_graph_value(value) every time
#[inline(always)]
pub fn from_graph_value<T: FromGraphValue>(value: &GraphValue) -> RedisResult<T> {
    FromGraphValue::from_graph_value(value)
}

/// Helper for creating Rediserror
pub fn create_rediserror(desc: &str) -> RedisError {
    (
        ErrorKind::TypeError,
        "Parsing Error",
        desc.to_owned()
    ).into()
}

#[macro_export]
macro_rules! query {
    ( $s:expr $(, $ro:literal)?) => {{
        #[allow(unused_assignments, unused_mut)]
        let mut read_only = false;
        $(
            read_only = $ro;
        )?
        crate::types::GraphQuery {
            query: $s, read_only, params: vec![]
        }
    }};
    ( $s:expr, { $( $k:expr => $v:expr ),* } $(, $ro:literal)?) => {{
        #[allow(unused_assignments, unused_mut)]
        let mut read_only = false;
        $(
            read_only = $ro;
        )?
        crate::types::GraphQuery {
            query: $s, read_only, params: vec![$(
                ($k, crate::parse::Parameter::from($v)),
            )*]
        }
    }}
}
