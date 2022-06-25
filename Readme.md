# redisgraphio

[![crates.io](https://img.shields.io/badge/crates.io-v0.4.2-orange)](https://crates.io/crates/redisgraphio)


This is a rust client library for working with the [RedisGraph](https://oss.redislabs.com/redisgraph) module for [Redis](https://redis.io/).\
It works in conjunction with [redis-rs](https://docs.rs/redis) by implementing a trait for the redis connection.

### Features
- [Async support](#asynchronous-usage)
- [Serialisation](https://docs.rs/redisgraphio/latest/redisgraphio/trait.FromGraphValue.html) into custom types
- Query parameter escaping (See below)


## Synchronous usage
```toml
[dependencies]
redis = "0.21" # or higher
redisgraphio = "0.1"
```
 
```rust
use redis::RedisResult;
use redisgraphio::*;

fn rider_example() -> RedisResult<()> {
    let client = redis::Client::open("redis://127.0.0.1/")?;
    let mut con = client.get_connection()?;
    con.graph_query_void("my_graph", query!(
        "CREATE (:Rider {name:'Valentino Rossi'})-[:rides]->(:Team {name:'Yamaha'})"
    ))?;
    // Assuming this could be malicious user input you need to escape
    let team_name = "Yamaha";
    let riders: Vec<(Node,)> = con.graph_query(
        "my_graph", // graph to query
        query!(
            "MATCH (rider:Rider)-[:rides]->(:Team {name: $team}) RETURN rider",
            {
                "team" => team_name
            },
            true // Optinal parameter to enable read only access, default is false
    )
    )?.data;
    for (rider,) in riders {
        let name: String = rider.get_property_by_index(0)?;
        println!("{name}")
    }
    Ok(())
}
```


## Asynchronous usage

To enable the redisgraphio async commands either enable the `tokio-comp` or `async-std-comp`
```toml
[dependencies]
redis = "0.21.0"
redis-graph = { version = "0.1", features = ['tokio-comp'] }
```

```rust
use redis::RedisResult;
use redisgraphio::*;

async fn rider_example() -> RedisResult<()> {
    let client = redis::Client::open("redis://127.0.0.1/")?;
    let mut con = client.get_async_connection().await?;
    con.graph_query_void("my_graph", query!(
        "CREATE (:Rider {name:'Valentino Rossi'})-[:rides]->(:Team {name:'Yamaha'})"
    )).await?;
    // Assuming this could be malicious user input you need to escape
    let team_name = "Yamaha";
    let riders: Vec<(Node,)> = con.graph_query(
        "my_graph",
        query!(
            "MATCH (rider:Rider)-[:rides]->(:Team {name: $team}) RETURN rider",
            {
                "team" => team_name
            },
            true // Optinal parameter to enable read only access, default is false
    )
    ).await?.data;
    for (rider,) in riders {
        let name: String = rider.get_property_by_index(0)?;
        println!("{name}")
    }
    Ok(())
}
```


## Credit

The crates API was inspired by the [redis-graph](https://github.com/tompro/redis_graph) crate which also implents traits on the redis connection.\
The serialisation was inspired by [redis-rs](https://docs.rs/redis) itself.
