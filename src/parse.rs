use redis::{Value, RedisResult, FromRedisValue, from_redis_value};

use crate::helpers::create_rediserror;

/// Official enum from redis-graph https://github.com/RedisGraph/RedisGraph/blob/master/src/resultset/formatters/resultset_formatter.h#L20-L33
mod Types {
    pub const VALUE_UNKNOWN: i64 = 0;
	pub const VALUE_NULL: i64 = 1;
	pub const VALUE_STRING: i64 = 2;
	pub const VALUE_INTEGER: i64 = 3;
	pub const VALUE_BOOLEAN: i64 = 4;
	pub const VALUE_DOUBLE: i64 = 5;
	pub const VALUE_ARRAY: i64 = 6;
	pub const VALUE_EDGE: i64 = 7;
	pub const VALUE_NODE: i64 = 8;
	pub const VALUE_PATH: i64 = 9;
	pub const VALUE_MAP: i64 = 10;
	pub const VALUE_POINT: i64 = 11;
}

#[derive(Clone, Debug, PartialEq)]
pub enum GraphValue {
    Scalar(Value),
    Node(Value),
    Array(Vec<GraphValue>),
    Integer(i64),
    Double(f64),
    String(String),
    Boolean(bool),
    Null,
    Relation(Value),
}

/*
bulk(bulk(bulk(int(1), string-data('"4"')), bulk(int(1), string-data('"7"'))), bulk(bulk(bulk(int(3), int(4)), bulk(int(3), int(7)))), bulk(string-data('"Cached execution: 1"')
bulk(
    # Header
    bulk(
        bulk(
            int(1), string-data('"4"')
        ),
        bulk(
            int(1), string-data('"7"')
        )
    ),
    # Body
    bulk(
        # Matches
        bulk(
            # Returns
            bulk(
                int(3), int(4)
            ), 
            bulk(
                int(3), int(7)
            )
        )
    ),
    # Metadata
    bulk(
        string-data('"Cached execution: 1"'),
        string-data('"Query internal execution time: 0.109212 milliseconds"')
    ))
*/ 

pub trait FromGraphValue: Sized {
    fn from_graph_value(value: GraphValue) -> RedisResult<Self>;
}


/// Helper macro to apply a macro to each following type
macro_rules! apply_macro {
    ($m:tt, $($x:ty),+) => {
        $(
            $m!($x);
        )*
    };
}

/// Macro for implementing the FromGraphValue Trait for a int type
macro_rules! from_graph_value_for_int {
    ( $t:ty ) => {
        impl FromGraphValue for $t {
            fn from_graph_value(value: GraphValue) -> RedisResult<Self> {
                match value {
                    GraphValue::Integer(val) => Ok(val as $t),
                    _ => Err(create_rediserror(&format!(concat!("Cant convert {:?} to ", stringify!($t)), value)))
                }
            }
        }
    };
}
/// Macro for implementing the FromGraphValue Trait for a float type
macro_rules! from_graph_value_for_float {
    ( $t:ty ) => {
        impl FromGraphValue for $t {
            fn from_graph_value(value: GraphValue) -> RedisResult<Self> {
                match value {
                    GraphValue::Double(val) => Ok(val as $t),
                    _ => Err(create_rediserror(&format!(concat!("Cant convert {:?} to ", stringify!($t)), value)))
                }
            }
        }
    };
}
apply_macro!(from_graph_value_for_int, i8, i16, i32, i64, u8, u16, u32, u64);
apply_macro!(from_graph_value_for_float, f32, f64);

impl FromGraphValue for bool {
    fn from_graph_value(value: GraphValue) -> RedisResult<Self> {
        match value {
            GraphValue::Boolean(val) => Ok(val),
            _ => Err(create_rediserror(&format!("Cant convert {:?} to bool", value)))
        }
    }
}

impl <T: FromGraphValue>FromGraphValue for Vec<T> {
    fn from_graph_value(value: GraphValue) -> RedisResult<Self> {
        match value {
            GraphValue::Array(val) => Ok(
                val.into_iter()
                    .map(FromGraphValue::from_graph_value)
                    .collect::<RedisResult<Self>>()?),
            _ => Err(create_rediserror(&format!("Cant convert {:?} to Vec", value)))
        }
    }
}

impl FromGraphValue for GraphValue {
    fn from_graph_value(value: GraphValue) -> RedisResult<Self> {
        Ok(value)
    }
}

impl FromGraphValue for String {
    fn from_graph_value(value: GraphValue) -> RedisResult<Self> {
        match value {
            GraphValue::String(s) => Ok(s),
            _ => Err(create_rediserror(&format!("Cant convert {:?} to String", value)))
        }
    }
}

/// This is copied and modified from the rust redis lib and modified for Graphvalue
macro_rules! from_graph_value_for_tuple {
    () => ();
    ($($name:ident,)+) => (
        #[doc(hidden)]
        impl<$($name: FromGraphValue),+> FromGraphValue for ($($name,)*) {
            // we have local variables named T1 as dummies and those
            // variables are unused.
            #[allow(non_snake_case, unused_variables)]
            fn from_graph_value(v: GraphValue) -> RedisResult<($($name,)*)> {
                match v {
                    GraphValue::Array(mut items) => {
                        // hacky way to count the tuple size
                        let mut n = 0;
                        $(let $name = (); n += 1;)*
                        if items.len() != n {
                            return Err(create_rediserror(&format!("Wrong length to create Tuple from {:?}", &items)))
                        }

                        Ok(($({
                            let $name = ();
                            FromGraphValue::from_graph_value(items.remove(0))?
                        },)*))
                    }
                    _ => Err(create_rediserror(&format!("Can not create Tuple from {:?}", v)))
                }
            }
        }
        from_graph_value_for_tuple_peel!($($name,)*);
    )
}

/// This chips of the leading one and recurses for the rest. So if the first
/// iteration was T1, T2, T3 it will recurse to T2, T3. It stops for tuples
/// of size 1 (does not implement down to unit).
macro_rules! from_graph_value_for_tuple_peel {
    ($name:ident, $($other:ident,)*) => (from_graph_value_for_tuple!($($other,)*);)
}

from_graph_value_for_tuple! { T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, }

impl FromRedisValue for GraphValue {
    fn from_redis_value(v: &Value) -> RedisResult<Self> {
        use Types::*;
        match v {
            Value::Bulk(data) if data.len() == 2 => match data[0] {
                Value::Int(VALUE_NODE) => Ok(GraphValue::Node(from_redis_value(&data[1])?)),
                Value::Int(VALUE_EDGE) => Ok(GraphValue::Relation(from_redis_value(&data[1])?)),
                Value::Int(VALUE_NULL) => Ok(GraphValue::Null),
                Value::Int(VALUE_DOUBLE) => Ok(GraphValue::Double(from_redis_value(&data[1])?)),
                Value::Int(VALUE_INTEGER) => Ok(GraphValue::Integer(from_redis_value(&data[1])?)),
                Value::Int(VALUE_ARRAY) => Ok(GraphValue::Array(from_redis_value(&data[1])?)),
                Value::Int(VALUE_STRING) => Ok(GraphValue::String(from_redis_value(&data[1])?)),
                Value::Int(VALUE_BOOLEAN) => Ok(GraphValue::Boolean({
                    match from_redis_value::<String>(&data[1])?.as_str() {
                        "true" => true,
                        "false" => false,
                        _ => return Err(create_rediserror(&format!("Cant convert {:?} to bool", &data[1])))
                    }
                })),
                _ => Ok(GraphValue::Scalar(data[1].to_owned())),
            }
            value => Err(create_rediserror(
                &format!("Couldnt convert {:?} to GraphValue", value)
            ))
        }
    }
}

#[derive(Clone, PartialEq, Debug)]
pub enum Parameter {
    String(String),
    Int(i64),
    Double(f64)
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

