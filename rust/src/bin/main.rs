use xain_fl::coordinator_async::State;

#[tokio::main]
async fn main() {
    let (tx, mut state) = State::new().unwrap();
    tx.send(());

    loop {
        state = state.next().await;
    }
}
