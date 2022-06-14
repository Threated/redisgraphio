use redis::{RedisResult, ErrorKind, RedisError};

use crate::{FromGraphValue, GraphValue};


/// Helper macro to apply a macro to each following type
macro_rules! apply_macro {
    ($m:tt, $($x:ty),+) => {
        $(
            $m!($x);
        )*
    };
}

pub(crate) use apply_macro;

/// Shorthand for FromGraphValue::from_graph_value(value)
#[inline(always)]
pub fn from_graph_value<T: FromGraphValue>(value: GraphValue) -> RedisResult<T> {
    FromGraphValue::from_graph_value(value)
}

/// Helper for creating a Rediserror
pub fn create_rediserror(desc: &str) -> RedisError {
    (
        ErrorKind::TypeError,
        "Parsing Error",
        desc.to_owned()
    ).into()
}

/// Macro for creating a GraphQuery
/// ## Diffrent usecases
/// ```
/// query!("query string"); // Normal query
/// query!("query string", true); // Normal read only query
/// query!(
///     "query string $param",
///     {
///         "param" => 5 // or "Some string" or 4.8 everything is converted with Parameter::from
///     }
/// ); // Query with parameters and read only
/// query!(
///     "query string $param $name",
///     {
///         "param" => 5,
///         "name" => "username"
///     },
///     true
/// ); // Query with parameters and read only
/// ```
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
                ($k, crate::types::Parameter::from($v)),
            )*]
        }
    }}
}
