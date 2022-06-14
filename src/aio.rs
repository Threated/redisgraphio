use redis::{aio::ConnectionLike, RedisFuture, cmd};

use crate::{types::GraphQuery, FromGraphValue, GraphResponse, query};


/// Implements redis graph related commands for an asynchronous connection
pub trait AsyncGraphCommands: ConnectionLike + Send + Sized {
    /// Send a graph query asynchronously
    fn graph_query<'a, Q, RT>(
        &'a mut self,
        graph: &'a str,
        query: Q,
    ) -> RedisFuture<GraphResponse<RT>>
    where 
        Q: Into<GraphQuery> + Send + 'a,
        RT: FromGraphValue
    {
        Box::pin(async move {
            let query = query.into();
            cmd(query.read_type())
                .arg(graph)
                .arg(query.construct_query())
                .arg("--compact")
                .query_async(self)
                .await
        })
    }

    /// Send a graph query asynchronously and ignore the result data
    fn graph_query_void<'a, Q>(
        &'a mut self,
        graph: &'a str,
        query: Q,
    ) -> RedisFuture<GraphResponse<()>>
    where 
        Q: Into<GraphQuery> + Send + 'a,
    {
        Box::pin(async move {
            let query = query.into();
            cmd(query.read_type())
                .arg(graph)
                .arg(query.construct_query())
                .arg("--compact")
                .query_async(self)
                .await
        })
    }

    /// Returns a vector where the index is a label id and the value at that index is the corresponding label name
    fn labels<'a>(&'a mut self, graph: &'a str) -> RedisFuture<'a, Vec<String>> {
        Box::pin(async move {
            let data: Vec<Vec<String>> = self.graph_query(graph, query!("CALL db.labels()")).await?.data;
            Ok(data.into_iter().map(|mut vec| vec.remove(0)).collect())
        })
    }

    /// Returns a vector where the index is a property key id and the value at that index is the corresponding property key name
    fn property_keys<'a>(&'a mut self, graph: &'a str) -> RedisFuture<'a, Vec<String>> {
        Box::pin(async move {
            let data: Vec<Vec<String>> = self.graph_query(graph, query!("CALL db.propertyKeys()")).await?.data;
            Ok(data.into_iter().map(|mut vec| vec.remove(0)).collect())
        })
    }

    /// Returns a vector where the index is a relationship id and the value at that index is the corresponding relationship name
    fn relationship_types<'a>(&'a mut self, graph: &'a str) -> RedisFuture<'a, Vec<String>> {
        Box::pin(async move {
            let data: Vec<Vec<String>> = self.graph_query(graph, query!("CALL db.relationshipTypes()")).await?.data;
            Ok(data.into_iter().map(|mut vec| vec.remove(0)).collect())
        })
    }
}

impl<T> AsyncGraphCommands for T where T: Send + ConnectionLike {}
