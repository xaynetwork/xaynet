use tokio::sync::oneshot;
use tracing_subscriber::*;
use xain_fl::{
    crypto::{generate_encrypt_key_pair, generate_signing_key_pair},
    state_machine::{
        requests::{Request, SumRequest},
        StateMachine,
    },
    PetError,
};

#[tokio::main]
async fn main() {
    let _fmt_subscriber = FmtSubscriber::builder()
        .with_env_filter(EnvFilter::from_default_env())
        .with_ansi(true)
        .init();

    let (request_tx, mut state_machine) = StateMachine::new().unwrap();

    tokio::spawn(async move {
        loop {
            state_machine = state_machine.next().await;
        }
    });

    let (tx, rx) = oneshot::channel::<Result<(), PetError>>();
    let (pk, _) = generate_signing_key_pair();
    let (ephm_pk, _) = generate_encrypt_key_pair();
    let sum_req = SumRequest {
        participant_pk: pk,
        ephm_pk: ephm_pk,
        response_tx: tx,
    };

    let _ = request_tx.send(Request::Sum(sum_req));
    println!("{:?}", rx.await);
}
