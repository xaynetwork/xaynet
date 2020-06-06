use tracing_subscriber::*;
use xain_fl::coordinator_async::StateMachine;

#[tokio::main]
async fn main() {
    let _fmt_subscriber = FmtSubscriber::builder()
        .with_env_filter(EnvFilter::from_default_env())
        .with_ansi(true)
        .init();

    let (tx, mut state) = StateMachine::new().unwrap();
    tx.send(vec![12]);

    loop {
        state = state.next().await;
    }
}
