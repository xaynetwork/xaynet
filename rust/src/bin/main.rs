use tracing_subscriber::*;
use xain_fl::coordinator_async::{redis::store::RedisStore, StateMachine};
#[tokio::main]
async fn main() {
    let _fmt_subscriber = FmtSubscriber::builder()
        .with_env_filter(EnvFilter::from_default_env())
        .with_ansi(true)
        .init();
    let redis = RedisStore::new("redis://127.0.0.1/", 10).await.unwrap();

    let (tx, mut state) = StateMachine::new(redis).unwrap();
    tx.send(vec![12]);

    loop {
        state = state.next().await;
    }
}
