use tracing_subscriber::*;
use xain_fl::coordinator_async::{store::client::RedisStore, StateMachine};
#[tokio::main]
async fn main() {
    let _fmt_subscriber = FmtSubscriber::builder()
        .with_env_filter(EnvFilter::from_default_env())
        .with_ansi(true)
        .init();
    let redis = RedisStore::new("redis://127.0.0.1/", 10).await.unwrap();

    let (message_tx, events_rx, mut state) = StateMachine::new(redis).unwrap();
    message_tx.send(vec![12]);

    loop {
        state = state.next().await;
    }
}
