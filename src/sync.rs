use crate::{GraphResponse, FromGraphValue, GraphQuery, query};
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

    fn graph_query_void<Q>(
        &mut self,
        graph: &str,
        query: Q,
    ) -> RedisResult<GraphResponse<()>> where Q: Into<GraphQuery> {
        let query = query.into();
        cmd(query.read_type())
            .arg(graph)
            .arg(query.construct_query())
            .arg("--compact")
            .query(self)
    }

    fn labels(&mut self, graph: &str) -> RedisResult<Vec<String>> {
        let data: Vec<Vec<String>> = self.graph_query(graph, query!("CALL db.labels()"))?.data;
        Ok(data.into_iter().map(|mut vec| vec.remove(0)).collect())
    }

    fn property_keys(&mut self, graph: &str) -> RedisResult<Vec<String>> {
        let data: Vec<Vec<String>> = self.graph_query(graph, query!("CALL db.propertyKeys()"))?.data;
        Ok(data.into_iter().map(|mut vec| vec.remove(0)).collect())
    }

    fn relationship_types(&mut self, graph: &str) -> RedisResult<Vec<String>> {
        let data: Vec<Vec<String>> = self.graph_query(graph, query!("CALL db.relationshipTypes()"))?.data;
        Ok(data.into_iter().map(|mut vec| vec.remove(0)).collect())
    }
}

impl<T> GraphCommands for T where T: ConnectionLike {}

