use redis::{aio::ConnectionLike, RedisFuture, cmd};

use crate::{types::GraphQuery, FromGraphValue, GraphResponse};



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

}

impl<T> AsyncGraphCommands for T where T: Send + ConnectionLike {}
