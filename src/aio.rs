use redis::{aio::ConnectionLike, RedisFuture, cmd};

use crate::{types::GraphQuery, FromGraphValue, GraphResponse, query};



pub trait AsyncGraphCommands: ConnectionLike + Send + Sized {
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

    fn labels<'a>(&'a mut self, graph: &'a str) -> RedisFuture<'a, Vec<String>> {
        Box::pin(async move {
            let data: Vec<Vec<String>> = self.graph_query(graph, query!("CALL db.labels()")).await?.data;
            Ok(data.into_iter().map(|mut vec| vec.remove(0)).collect())
        })
    }

    fn property_keys<'a>(&'a mut self, graph: &'a str) -> RedisFuture<'a, Vec<String>> {
        Box::pin(async move {
            let data: Vec<Vec<String>> = self.graph_query(graph, query!("CALL db.propertyKeys()")).await?.data;
            Ok(data.into_iter().map(|mut vec| vec.remove(0)).collect())
        })
    }

    fn relationship_types<'a>(&'a mut self, graph: &'a str) -> RedisFuture<'a, Vec<String>> {
        Box::pin(async move {
            let data: Vec<Vec<String>> = self.graph_query(graph, query!("CALL db.relationshipTypes()")).await?.data;
            Ok(data.into_iter().map(|mut vec| vec.remove(0)).collect())
        })
    }
}

impl<T> AsyncGraphCommands for T where T: Send + ConnectionLike {}
