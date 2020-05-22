use tokio::stream::StreamExt;
use tracing::{info, Level};
use tracing_subscriber;

#[tokio::main]
async fn main() -> redis::RedisResult<()> {
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    let client = redis::Client::open("redis://127.0.0.1/").unwrap();
    let mut pubsub_conn = client.get_async_connection().await?.into_pubsub();

    pubsub_conn.subscribe("message").await?;

    // pub/sub messages are never stored on redis. This means that if the subscriber cannot be
    // reached, the messages have disappeared forever.
    // You can try it out by stopping and starting the subscriber while the publisher publishes
    // new messages.
    let mut msg_stream = pubsub_conn.on_message();
    loop {
        if let Some(msg) = msg_stream.next().await {
            info!("New message {:?}", msg.get_payload::<String>().unwrap());
        }
    }
}
