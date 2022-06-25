use crate::{
    query, GeoPoint, GraphCommands, GraphMap, GraphPath, GraphQuery, GraphValue,
    PropertyAccess, GraphStatistic
};

use paste::paste;

#[test]
fn test_query_macro() {
    assert_eq!(
        query!("Return 1"),
        GraphQuery {
            query: "Return 1",
            read_only: false,
            params: vec![]
        }
    );
    assert_eq!(
        query!("Return 1", true),
        GraphQuery {
            query: "Return 1",
            read_only: true,
            params: vec![]
        }
    );
    assert_eq!(
        query!("Return 1", {
            "a" => 4,
            "b" => "test"
        }),
        GraphQuery {
            query: "Return 1",
            read_only: false,
            params: vec![("a", 4.into()), ("b", "test".into())]
        }
    );
    assert_eq!(
        query!("Return 1", {
            "a" => 4.5,
            "b" => "test"
        }, true),
        GraphQuery {
            query: "Return 1",
            read_only: true,
            params: vec![("a", 4.5.into()), ("b", "test".into())]
        }
    );
}

fn get_client() -> redis::Client {
    redis::Client::open("redis://localhost:6379/").unwrap()
}

#[cfg(any(feature = "tokio-comp", feature = "async-std-comp"))]
async fn async_con() -> redis::aio::Connection {
    get_client().get_async_connection().await.unwrap()
}

fn sync_con() -> redis::Connection {
    get_client().get_connection().unwrap()
}

#[cfg(feature = "tokio-comp")]
fn tokio_runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .build()
        .unwrap()
}

macro_rules! test_parse {
    ($name:ident, $query:expr, {$($types:ty => $values:expr),+}) => {
        paste! {
            #[test]
            fn [<parse_ $name>]() {
                let data: Vec<($($types,)*)> = sync_con().graph_query("test", $query).unwrap().data;
                assert_eq!(data[0], ($($values,)*));
            }

            #[test]
            #[cfg(feature = "tokio-comp")]
            fn [<parse_ $name _tokio>]() {
                use crate::AsyncGraphCommands;
                let data: Vec<($($types,)*)> = tokio_runtime().block_on(async move {
                    async_con().await.graph_query("test", $query).await.unwrap().data
                });
                assert_eq!(data[0], ($($values,)*));
            }

            #[test]
            #[cfg(feature = "async-std-comp")]
            fn [<parse_ $name _async_std>]() {
                use crate::AsyncGraphCommands;
                let data: Vec<($($types,)*)> = async_std::task::block_on(async move {
                    async_con().await.graph_query("test", $query).await.unwrap().data
                });
                assert_eq!(data[0], ($($values,)*));
            }
        }
    };
}

test_parse! {ints,
    query!("Return 1, 2, 3, 4, 5, 6, 7, 18446744073709551617"), // 2**64+1 == 18446744073709551617
    {
        u8 => 1,
        u16 => 2,
        u32 => 3,
        u64 => 4,
        i8 => 5,
        i16 => 6,
        i32 => 7,
        i64 => 0x7fffffffffffffff // Redis only allows 64bit ints so this is the expected interger overflow value
    }
}

test_parse! {double,
    query!("Return 1.0, 3.3, 4.67"),
    {
        f32 => 1.0,
        f32 => 3.3,
        f64 => 4.67
    }
}

test_parse! {boolean,
    query!("Return 1.0 = 1.0, 0=1, true"),
    {
        bool => true,
        bool => false,
        bool => true
    }
}

test_parse! {vec,
    query!("Return [1, 2, 3, 4], [5, 6]"),
    {
        Vec<i32> => vec![1, 2, 3, 4],
        Option<Vec<i32>> => Some(vec![5, 6])
    }
}

test_parse! {null,
    query!("Return null, null as b"),
    {
        GraphValue => GraphValue::Null,
        Option<i32> => None
    }
}

test_parse! {string,
    query!(r#"Return "test", 'test', $other, $a"#, {
        "other" => r#"a\"b\'c'd"e"#,
        "a" => r#"\" Return 1337//"#
    }),
    {
        String => "test".to_string(),
        String => "test".to_string(),
        String => r#"a\"b\'c'd"e"#.to_string(),
        String => r#"\" Return 1337//"#.to_string()
    }
}

test_parse! {map,
    query!("Return {a: 5, b: 4.5, c: [1,2]}"),
    {
        GraphMap => GraphMap([
            ("a".to_string(), GraphValue::Integer(5)),
            ("b".to_string(), GraphValue::Double(4.5)),
            ("c".to_string(), GraphValue::Array(vec![
                GraphValue::Integer(1),
                GraphValue::Integer(2)
            ]))
        ].into_iter().collect())
    }
}

test_parse! {point,
    query!("Return point({latitude: 32.070794860, longitude: 34.820751118})"),
    {
        GeoPoint => GeoPoint {
            latitude: 32.070794860,
            longitude: 34.820751118
        }
    }
}

#[test]
fn test_parse_graphtypes() {
    let con = &mut sync_con();
    con.graph_query_void("test", query!("Create (:User {a: 1})-[:follows]->(:User)"))
        .unwrap();
    let paths: Vec<(GraphPath,)> = con
        .graph_query(
            "test",
            query!("Match p=(a:User {a: 1})-[:follows {b:3}]->(b:User) Return p"),
        )
        .unwrap()
        .data;
    for (GraphPath {
        nodes,
        relationships,
    },) in paths.into_iter()
    {
        assert_eq!(nodes[0].get_property_by_index::<i32>(0).unwrap(), 1);
        assert_eq!(nodes[0].property_values::<(i32,)>().unwrap(), (1,));
        assert_eq!(relationships[0].get_property_by_index::<i32>(0).unwrap(), 3);
    }

    let res = con.graph_query_void(
        "test",
        query!("Match (a:User {a: 1})-[:follows]->(b:User) Detach Delete a, b"),
    )
    .unwrap();

    assert!(res.get_statistic(GraphStatistic::ExecutionTime).is_some());
    assert!(res.get_statistic(GraphStatistic::CachedExecution).is_some());
    assert_eq!(res.get_statistic(GraphStatistic::NodesDeleted), Some(2.0));
    assert_eq!(res.get_statistic(GraphStatistic::RelationshipsDeleted), Some(1.0));
}

#[test]
fn test_mappings() {
    let con = &mut sync_con();
    con.labels("test").unwrap();
    con.relationship_types("test").unwrap();
    con.property_keys("test").unwrap();
}
