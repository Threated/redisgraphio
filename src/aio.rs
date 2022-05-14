use redis::{aio::ConnectionLike, RedisFuture, cmd};

use crate::{GraphValue, query::GraphQuery};



pub trait AsyncGraphCommands: ConnectionLike + Send + Sized {
    fn graph_query<'a, Q: Into<GraphQuery> + Send + 'a>(
        &'a mut self,
        graph: &'a str,
        query: Q,
    ) -> RedisFuture<GraphValue> {
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
