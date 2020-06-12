use xain_fl::state_machine::StateMachine;
use xain_fl::state_machine::requests::SumRequest;
use xain_fl::state_machine::requests::Request;
use xain_fl::PetError;
use xain_fl::crypto::generate_encrypt_key_pair;
use xain_fl::crypto::generate_signing_key_pair;
use tracing_subscriber::*;
use tokio::sync::oneshot;

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
    let (pk, _ ) = generate_signing_key_pair();
    let (ephm_pk, _ ) = generate_encrypt_key_pair();
    let sum_req = SumRequest{
         participant_pk: pk,
         ephm_pk: ephm_pk,
         response_tx: tx,
    };
    
    let _ = request_tx.send(Request::Sum(sum_req));
    println!("{:?}",rx.await);
}




