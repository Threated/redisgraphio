use redis::RedisResult;

use crate::{FromGraphValue, GraphValue};


/// So you dont have to write FromGraphValue::from_graph_value(value) every time
pub fn from_graph_value<T: FromGraphValue>(value: &GraphValue) -> RedisResult<T> {
    FromGraphValue::from_graph_value(value)
}

/// Helper macro to to simply create Parameters for the query
#[macro_export]
macro_rules! params {
    ( $( $x:ident ),* ) => {
        {
            &[
                $(
                    (stringify!($x), $x.into()),
                )*
            ]
        }
    };
}
