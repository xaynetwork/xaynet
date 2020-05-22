use redis::AsyncCommands;
use tokio::stream::StreamExt;
use tracing::{info, Level};
use tracing_subscriber;

// implementation of a client-side-caching example
// https://redis.io/topics/client-side-caching

#[tokio::main]
async fn main() -> redis::RedisResult<()> {
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    let client = redis::Client::open("redis://127.0.0.1/").unwrap();

    // channel used for receiving invalidation notifications
    let mut inval_chan = client.get_async_connection().await?;
    let id: u32 = redis::cmd("CLIENT")
        .arg("ID")
        .query_async(&mut inval_chan)
        .await?;
    let mut inval_chan = inval_chan.into_pubsub();
    inval_chan.subscribe("__redis__:invalidate").await?;

    // channel used for getting data
    let mut data_chan = client.get_async_connection().await?;
    // First write some dummy data that we can cache
    info!("Set \"foo\" = \"abc\"");
    redis::cmd("SET")
        .arg(&["foo", "abc"])
        .query_async(&mut data_chan)
        .await?;
    // enable tracking and redirect the invalidation notifications into the invalidation channel
    // with the new version of the Redis protocol, RESP3, it is possible to run the data queries and
    // receive the invalidation messages in the same connection.
    // Support for RESP3 is not implemented for this redis lib yet.
    redis::cmd("CLIENT")
        .arg(&["TRACKING", "on", "REDIRECT", &id.to_string()[..]])
        .query_async(&mut data_chan)
        .await?;
    // get the value of "foo"
    redis::cmd("GET")
        .arg("foo")
        .query_async(&mut data_chan)
        .await?;
    // at this point Redis knows that the client caches the value of "foo"

    // change the value of "foo"
    tokio::spawn(other_client_modifies_foo());

    info!("Listen for notifications");
    let mut inval_stream = inval_chan.on_message();
    loop {
        if let Some(notification) = inval_stream.next().await {
            // the response is a bulk type: bulk(string-type)
            let key = &notification.get_payload::<Vec<String>>().unwrap()[0];
            info!("Invalidation notification for {:?}", key);
            let updated_value: String = redis::cmd("GET")
                .arg(key)
                .query_async(&mut data_chan)
                .await?;
            info!("Update value of {:?} is: {:?}", key, updated_value);
        }
    }
}

async fn other_client_modifies_foo() {
    use std::time::Duration;
    use tokio::time::delay_for;

    delay_for(Duration::from_secs(2)).await;
    let client = redis::Client::open("redis://127.0.0.1/").unwrap();
    let mut con = client.get_async_connection().await.unwrap();
    info!("Value of \"foo\" is modified");
    con.set::<_, _, String>("foo", "cba").await.unwrap();
}
