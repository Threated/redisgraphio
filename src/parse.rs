use indexmap::IndexMap;
use redis::{from_redis_value, FromRedisValue, RedisResult, Value};
use std::{
    collections::HashMap,
};

use crate::{
    from_graph_value,
    helpers::{create_rediserror, apply_macro}
};

/// [Official enum](https://github.com/RedisGraph/RedisGraph/blob/master/src/resultset/formatters/resultset_formatter.h#L20-L33) from redis-graph 
mod types {
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

/// An enum containing every possible type that can be returned by redisgraph
#[derive(Clone, Debug, PartialEq)]
pub enum GraphValue {
    /// Value is Unknown and stored as a [redis::Value]
    Unknown(Value),
    /// A Map as returned by
    /// ```cypher
    /// Return {a: 2, b: "Hello"}
    /// ```
    Map(GraphMap),
    /// A Point as returned by
    /// ```cypher
    /// Return point({latitude: 32.070794860, longitude: 34.820751118})
    /// ```
    Point(GeoPoint),
    /// A Path as returned by 
    /// ```cypher
    /// Match p=(:A)-[:B]->(:C) Return p
    /// ```
    Path(GraphPath),
    /// A Node as returned by 
    /// ```cypher
    /// Match (a:A) Return a
    /// ```
    Node(Node),
    /// A Relationship as returned by 
    /// ```cypher
    /// Match (:A)-[b:B]->(:C) Return b
    /// ```
    Relation(Relationship),
    /// A Array as returned by 
    /// ```cypher
    /// Return [1, 2.0, "Hi"]
    /// ```
    Array(Vec<GraphValue>),
    /// A Integer as returned by 
    /// ```cypher
    /// Return 1337
    /// ```
    Integer(i64),
    /// A Double as returned by 
    /// ```cypher
    /// Return 1337.0
    /// ```
    Double(f64),
    /// A String as returned by 
    /// ```cypher
    /// Return '1337'
    /// ```
    String(String),
    /// A Boolean as returned by 
    /// ```cypher
    /// Return true, false, 1=1
    /// ```
    Boolean(bool),
    /// A Null type which is returned when an Optinal Match does not match
    /// or when a property of a node is returned but the node does not have this property
    Null,
}

/// The type returned by the point method in cypher
#[derive(Debug, Clone, PartialEq)]
pub struct GeoPoint {
    /// latitude
    pub latitude: f32,
    /// longitude
    pub longitude: f32,
}

/// Map typed as returned by RETURN {a: 1}
#[derive(Debug, Clone, PartialEq)]
pub struct GraphMap(pub HashMap<String, GraphValue>);

impl GraphMap {
    /// Take ownership of the underlying HashMap 
    pub fn into_inner(self) -> HashMap<String, GraphValue> {
        self.0
    }

    /// Gets a value by its key and converts it to a given return type
    pub fn get<T: FromGraphValue>(&self, key: &str) -> RedisResult<Option<T>> {
        match self.0.get(key) {
            Some(val) => from_graph_value(val.clone()),
            None => Ok(None),
        }
    }
}

/// Node Type
#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    /// Redisgraph internal node id
    pub id: i64,
    /// Ids of the nodes labels that can be mapped to db.labels()
    pub label_ids: Vec<i64>,
    /// Map of property ids to property values can be mapped to db.propertyKeys()
    pub properties: IndexMap<i64, GraphValue>,
}

impl Node {
    /// Full constructor for Node
    pub fn new(id: i64, label_ids: Vec<i64>, properties: IndexMap<i64, GraphValue>) -> Self {
        Self {
            id,
            label_ids,
            properties,
        }
    }
}

/// Relationship Type
#[derive(Debug, Clone, PartialEq)]
pub struct Relationship {
    /// Redisgraph internal relationship id
    pub id: i64,
    /// Id of the relationships label that can be mapped to db.relationshipTypes()
    pub label_id: i64,
    /// Source Node Id
    pub src: i64,
    /// Destination Node
    pub dest: i64,
    /// Map of property ids to property values can be mapped by db.propertyKeys()
    pub properties: IndexMap<i64, GraphValue>,
}

impl Relationship {
    /// Full constructor for Relationship
    pub fn new(
        id: i64,
        label_id: i64,
        src: i64,
        dest: i64,
        properties: IndexMap<i64, GraphValue>,
    ) -> Self {
        Self {
            id,
            label_id,
            src,
            dest,
            properties,
        }
    }
}

/// Trait for unifying access to Node and Relationship properties
pub trait PropertyAccess {
    /// Returns a reference to the IndexMap containing the properties in order of definition and with the property key ids
    fn properties(&self) -> &IndexMap<i64, GraphValue>;

    /// get property by property label id
    fn get_property_by_label_id<T: FromGraphValue>(&self, label_id: i64) -> RedisResult<Option<T>> {
        match self.properties().get(&label_id) {
            Some(val) => from_graph_value(val.clone()),
            None => Ok(None),
        }
    }

    /// gets a property by its order of definition
    /// Note when relying on property order make sure every CREATE has the same order of these properties
    fn get_property_by_index<T: FromGraphValue>(&self, idx: usize) -> RedisResult<T> {
        from_graph_value(self.properties()[idx].clone())
    }

    /// get property values in the order they were defined
    fn property_values<T: FromGraphValue>(&self) -> RedisResult<T> {
        from_graph_value(GraphValue::Array(
            self.properties().values().cloned().collect(),
        ))
    }

    /// Same as `property_values()` but consumes the object taking ownership of the `Graphvalue`s
    fn into_property_values<T: FromGraphValue>(self) -> RedisResult<T>;
}

impl PropertyAccess for Node {
    #[inline(always)]
    fn properties(&self) -> &IndexMap<i64, GraphValue> {
        &self.properties
    }

    fn into_property_values<T: FromGraphValue>(self) -> RedisResult<T> {
        FromGraphValue::from_graph_value(GraphValue::Array(
            self.properties.into_values().collect(),
        ))
    }
}

impl PropertyAccess for Relationship {
    #[inline(always)]
    fn properties(&self) -> &IndexMap<i64, GraphValue> {
        &self.properties
    }

    fn into_property_values<T: FromGraphValue>(self) -> RedisResult<T> {
        FromGraphValue::from_graph_value(GraphValue::Array(
            self.properties.into_values().collect(),
        ))
    }
}

/// Type for graph paths as returned by MATCH p=(\:A)-[\:B]->(\:C) RETURN p
#[derive(Debug, PartialEq, Clone)]
pub struct GraphPath {
    /// Nodes of the GraphPath
    pub nodes: Vec<Node>,
    /// Relationships of the GraphPath
    pub relationships: Vec<Relationship>,
}

/// Trait for converting the response to an arbitray type which implents the trait
/// This is similar to the FromRedisValue trait from redis
/// 
/// ## Example
/// ```no_run
/// struct MyType {
///     a: i32
///     b: Vec<String>
/// }
/// 
/// impl FromGraphValue for MyType {
///     fn from_graph_value(value: GraphValue) -> RedisResult<Self> {
///         let (a, b): (i32, Vec<String>) = from_graph_value(value)?;
///         // You dont even need the type annotations above as they are inferred in this case
///         Ok(MyType {
///             a,
///             b
///         })
///     }
/// }
/// // Now you can write code like this
/// let con = // Connection to redis
/// let data: Vec<MyType> = con.graph_query("graphname", query!("RETURN 1, ['a', 'b']"))?.data;
/// ```
pub trait FromGraphValue: Sized {
    /// Converts the GraphValue to the implementing Type
    fn from_graph_value(value: GraphValue) -> RedisResult<Self>;
}


/// Macro for implementing the FromGraphValue Trait for a int type
macro_rules! from_graph_value_for_int {
    ( $t:ty ) => {
        impl FromGraphValue for $t {
            fn from_graph_value(value: GraphValue) -> RedisResult<Self> {
                match value {
                    GraphValue::Integer(val) => <$t>::try_from(val).map_err(|_| create_rediserror(concat!("Could not convert to ", stringify!($t)))),
                    _ => Err(create_rediserror(&format!(
                        concat!("Cant convert {:?} to ", stringify!($t)),
                        value
                    ))),
                }
            }
        }
    };
}

/// Macro for implementing the FromGraphValue Trait for a float type
impl FromGraphValue for f64 {
    fn from_graph_value(value: GraphValue) -> RedisResult<Self> {
        match value {
            GraphValue::Double(val) => Ok(val),
            _ => Err(create_rediserror(&format!(
                concat!("Cant convert {:?} to ", stringify!($t)),
                value
            ))),
        }
    }
}
apply_macro!(
    from_graph_value_for_int,
    i8,
    i16,
    i32,
    i64,
    u8,
    u16,
    u32,
    u64
);

impl FromGraphValue for bool {
    fn from_graph_value(value: GraphValue) -> RedisResult<Self> {
        match value {
            GraphValue::Boolean(val) => Ok(val),
            _ => Err(create_rediserror(&format!(
                "Cant convert {:?} to bool",
                value
            ))),
        }
    }
}

impl FromGraphValue for () {
    fn from_graph_value(_: GraphValue) -> RedisResult<Self> {
        Ok(())
    }
}

impl<T: FromGraphValue> FromGraphValue for Vec<T> {
    fn from_graph_value(value: GraphValue) -> RedisResult<Self> {
        match value {
            GraphValue::Array(val) => Ok(val
                .into_iter()
                .map(FromGraphValue::from_graph_value)
                .collect::<RedisResult<Self>>()?),
            _ => Err(create_rediserror(&format!(
                "Cant convert {:?} to Vec",
                value
            ))),
        }
    }
}

impl FromGraphValue for GraphMap {
    fn from_graph_value(value: GraphValue) -> RedisResult<Self> {
        match value {
            GraphValue::Map(map) => Ok(map),
            _ => Err(create_rediserror(&format!(
                "Cant convert {:?} to GraphMap",
                value
            ))),
        }
    }
}

impl FromGraphValue for GraphPath {
    fn from_graph_value(value: GraphValue) -> RedisResult<Self> {
        match value {
            GraphValue::Path(path) => Ok(path),
            _ => Err(create_rediserror(&format!(
                "Cant convert {:?} to GraphPath",
                value
            ))),
        }
    }
}

impl FromGraphValue for GeoPoint {
    fn from_graph_value(value: GraphValue) -> RedisResult<Self> {
        match value {
            GraphValue::Point(point) => Ok(point),
            _ => Err(create_rediserror(&format!(
                "Cant convert {:?} to GeoPoint",
                value
            ))),
        }
    }
}

impl FromGraphValue for Node {
    fn from_graph_value(value: GraphValue) -> RedisResult<Self> {
        match value {
            GraphValue::Node(node) => Ok(node),
            _ => Err(create_rediserror(&format!(
                "Cant convert {:?} to Node",
                value
            ))),
        }
    }
}

impl FromGraphValue for Relationship {
    fn from_graph_value(value: GraphValue) -> RedisResult<Self> {
        match value {
            GraphValue::Relation(rel) => Ok(rel),
            _ => Err(create_rediserror(&format!(
                "Cant convert {:?} to Relationship",
                value
            ))),
        }
    }
}

impl<T: FromGraphValue> FromGraphValue for Option<T> {
    fn from_graph_value(value: GraphValue) -> RedisResult<Self> {
        match value {
            GraphValue::Null => Ok(None),
            val => Ok(Some(from_graph_value(val)?)),
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
            GraphValue::String(s) => Ok(s.to_string()),
            _ => Err(create_rediserror(&format!(
                "Cant convert {:?} to String",
                value
            ))),
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
                            return Err(create_rediserror(&format!("Wrong length to create Tuple {} from {:?}", std::any::type_name::<Self>(), &items)))
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
        match v {
            Value::Bulk(data) if data.len() == 2 => match &data[0] {
                Value::Int(type_) => convert_to_graphvalue(*type_, &data[1]),
                value => Err(create_rediserror(&format!(
                    "Couldnt convert {:?} to GraphValue",
                    value
                ))),
            },
            value => Err(create_rediserror(&format!(
                "Couldnt convert {:?} to GraphValue",
                value
            ))),
        }
    }
}

impl FromRedisValue for GraphPath {
    fn from_redis_value(v: &Value) -> RedisResult<Self> {
        let (nodes, relationships): (GraphValue, GraphValue) = from_redis_value(v)?;
        Ok(GraphPath {
            nodes: from_graph_value(nodes)?,
            relationships: from_graph_value(relationships)?,
        })
    }
}

impl FromRedisValue for GeoPoint {
    fn from_redis_value(v: &Value) -> RedisResult<Self> {
        let (latitude, longitude): (f32, f32) = from_redis_value(v)?;
        Ok(GeoPoint {
            latitude,
            longitude,
        })
    }
}

impl FromRedisValue for GraphMap {
    fn from_redis_value(v: &Value) -> RedisResult<Self> {
        match v {
            Value::Bulk(values) => {
                let temp: Vec<(String, GraphValue)> = FromRedisValue::from_redis_values(values)?;
                Ok(GraphMap(temp.into_iter().collect()))
            }
            value => Err(create_rediserror(&format!(
                "Couldnt convert {:?} to GraphMap",
                value
            ))),
        }
    }
}

impl FromRedisValue for Node {
    fn from_redis_value(v: &Value) -> RedisResult<Self> {
        match v {
            Value::Bulk(ref values) if values.len() == 3 => Ok(Node::new(
                from_redis_value(&values[0])?,
                from_redis_value(&values[1])?,
                parse_properties(&values[2])?,
            )),
            val => Err(create_rediserror(&format!(
                "Couldnt convert {:?} to Node",
                val
            ))),
        }
    }
}


impl FromRedisValue for Relationship {
    fn from_redis_value(v: &Value) -> RedisResult<Self> {
        match v {
            Value::Bulk(ref values) if values.len() == 5 => Ok(Relationship::new(
                from_redis_value(&values[0])?,
                from_redis_value(&values[1])?,
                from_redis_value(&values[2])?,
                from_redis_value(&values[3])?,
                parse_properties(&values[4])?,
            )),
            val => Err(create_rediserror(&format!(
                "Couldnt convert {:?} to Relationship",
                val
            ))),
        }
    }
}

fn parse_properties(value: &Value) -> RedisResult<IndexMap<i64, GraphValue>> {
    // Same issue as in parse_header of Graphresponse
    let temp: Vec<Value> = from_redis_value(value)?;
    let properties: Vec<(i64, i64, Value)> = temp
        .into_iter()
        .map(|v| from_redis_value::<(i64, i64, Value)>(&v))
        .collect::<RedisResult<_>>()?;
    properties
        .into_iter()
        .map(
            |(property_id, type_, value)| match convert_to_graphvalue(type_, &value) {
                Ok(gvalue) => Ok((property_id, gvalue)),
                Err(err) => Err(err),
            },
        )
        .collect()
}

fn convert_to_graphvalue(type_: i64, val: &Value) -> RedisResult<GraphValue> {
    use types::*;
    match type_ {
        VALUE_NODE => Ok(GraphValue::Node(from_redis_value(val)?)),
        VALUE_EDGE => Ok(GraphValue::Relation(from_redis_value(val)?)),
        VALUE_PATH => Ok(GraphValue::Path(from_redis_value(val)?)),
        VALUE_MAP => Ok(GraphValue::Map(from_redis_value(val)?)),
        VALUE_POINT => Ok(GraphValue::Point(from_redis_value(val)?)),
        VALUE_NULL => Ok(GraphValue::Null),
        VALUE_DOUBLE => Ok(GraphValue::Double(from_redis_value(val)?)),
        VALUE_INTEGER => Ok(GraphValue::Integer(from_redis_value(val)?)),
        VALUE_ARRAY => Ok(GraphValue::Array(from_redis_value(val)?)),
        VALUE_STRING => Ok(GraphValue::String(from_redis_value(val)?)),
        VALUE_BOOLEAN => Ok(GraphValue::Boolean({
            // The FromRedisValue impl for bool does not support this conversion (for good reason)
            match from_redis_value::<String>(val)?.as_str() {
                "true" => true,
                "false" => false,
                _ => {
                    return Err(create_rediserror(&format!(
                        "Cant convert {:?} to bool",
                        val
                    )))
                }
            }
        })),
        VALUE_UNKNOWN | _ => Ok(GraphValue::Unknown(val.to_owned())),
    }
}
