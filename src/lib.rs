mod parse;
mod sync;
mod helpers;
mod types;

pub use crate::types::*;
pub use crate::sync::GraphCommands;
pub use crate::parse::*;
pub use crate::helpers::from_graph_value;


#[cfg(any(feature = "tokio-comp", feature = "async-std-comp"))]
mod aio;

#[cfg(any(feature = "tokio-comp", feature = "async-std-comp"))]
pub use crate::aio::AsyncGraphCommands;



#[cfg(test)]
mod tests {
    use crate::{sync::{GraphCommands}, query, GraphQuery};
    use paste::paste;

    #[test]
    fn test_query_macro() {
        assert_eq!(
            query!("Return 1"), 
            GraphQuery {
                query: "Return 1", read_only: false, params: vec![]
            }
        );
        assert_eq!(
            query!("Return 1", true), 
            GraphQuery {
                query: "Return 1", read_only: true, params: vec![]
            }
        );
        assert_eq!(
            query!("Return 1", {
                "a" => 4,
                "b" => "test"
            }), 
            GraphQuery {
                query: "Return 1", read_only: false, params: vec![
                    ("a", 4.into()),
                    ("b", "test".into())
                ]
            }
        );
        assert_eq!(
            query!("Return 1", {
                "a" => 4.5,
                "b" => "test"
            }, true), 
            GraphQuery {
                query: "Return 1", read_only: true, params: vec![
                    ("a", 4.5.into()),
                    ("b", "test".into())
                ]
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

    macro_rules! test_parse {
        ($type:ty) => {
            paste! {
                #[test]
                fn [<test_parse_ $type:lower>]() {
                    let data: Vec<(i32, i32, String)> = sync_con().graph_query("test", query!("Return 4, 7, 'test'")).unwrap().data;
                    dbg!(data);
                }

                #[test]
                #[cfg(any(feature = "tokio-comp", feature = "async-std-comp"))]
                fn [<test_parse_ $type:lower _async>]() {
    
                }
            }
        };
    }

    test_parse!(Scalar);
}
