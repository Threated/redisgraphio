use crate::{GraphResponse, FromGraphValue, GraphQuery};
use redis::{cmd, ConnectionLike, RedisResult};

pub trait GraphCommands: ConnectionLike + Sized {
    fn graph_query<Q, RT>(
        &mut self,
        graph: &str,
        query: Q,
    ) -> RedisResult<GraphResponse<RT>> where Q: Into<GraphQuery>, RT: FromGraphValue {
        let query = query.into();
        cmd(query.read_type())
            .arg(graph)
            .arg(query.construct_query())
            .arg("--compact")
            .query(self)
    }
}

impl<T> GraphCommands for T where T: ConnectionLike {}

