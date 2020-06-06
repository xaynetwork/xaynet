use xain_fl::coordinator_async::StateMachine;

#[tokio::main]
async fn main() {
    let (tx, mut state) = StateMachine::new().unwrap();
    tx.send(vec![12]);

    loop {
        state = state.next().await;
    }
}
