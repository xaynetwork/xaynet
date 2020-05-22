use redis::AsyncCommands;
use tracing::{info, Level};
use tracing_subscriber;

#[tokio::main]
async fn main() -> redis::RedisResult<()> {
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    let client = redis::Client::open("redis://127.0.0.1/").unwrap();
    let mut publish_conn = client.get_async_connection().await?;
    for i in 0..10_000 {
        info!("Send message {:?}", i);
        publish_conn.publish("message", i.to_string()).await?
    }
    Ok(())
}
