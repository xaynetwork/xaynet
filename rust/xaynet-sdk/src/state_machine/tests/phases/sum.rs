use thiserror::Error;
use xaynet_core::crypto::{ByteObject, EncryptKeyPair, EncryptKeySeed};

use crate::{
    state_machine::{
        tests::utils::{shared_state, SelectFor},
        IntoPhase,
        MockIO,
        Phase,
        SharedState,
        State,
        Sum,
    },
    unwrap_step,
};

/// Instantiate a sum phase.
fn make_phase(io: MockIO) -> Phase<Sum> {
    let shared = shared_state(SelectFor::Sum);
    let sum = make_sum(&shared);

    // Check IntoPhase<Sum> implementation
    let mut mock = MockIO::new();
    mock.expect_notify_sum().times(1).return_const(());
    let mut phase: Phase<Sum> = State::new(shared, sum).into_phase(Box::new(mock));

    // Set `phase.io` to the mock the test wants to use. Note that this drops the `mock` we
    // created above, so the expectations we set on `mock` run now.
    let _ = std::mem::replace(&mut phase.io, Box::new(io));
    phase
}

fn make_sum(shared: &SharedState) -> Sum {
    let ephm_keys = EncryptKeyPair::derive_from_seed(&EncryptKeySeed::zeroed());
    let sk = &shared.keys.secret;
    let seed = shared.round_params.seed.as_slice();
    let signature = sk.sign_detached(&[seed, b"sum"].concat());
    Sum {
        ephm_keys,
        sum_signature: signature,
        message: None,
    }
}

async fn check_step_1() -> Phase<Sum> {
    let io = MockIO::new();
    let phase = make_phase(io);
    let phase = unwrap_step!(phase, complete, sum);
    assert!(phase.state.private.message.is_some());
    phase
}

#[tokio::test]
async fn test_phase() {
    let mut phase = check_step_1().await;

    let mut io = MockIO::new();
    io.expect_send_message().times(1).returning(|_| Ok(()));
    let _ = std::mem::replace(&mut phase.io, Box::new(io));

    let _phase = unwrap_step!(phase, complete, sum2);
}

#[derive(Error, Debug)]
#[error("error")]
struct DummyErr;

#[tokio::test]
async fn test_send_sum_message_fails() {
    let mut phase = check_step_1().await;

    let mut io = MockIO::new();
    io.expect_send_message()
        .times(1)
        .returning(|_| Err(Box::new(DummyErr)));
    io.expect_notify_idle().times(1).return_const(());
    let _ = std::mem::replace(&mut phase.io, Box::new(io));

    let _phase = unwrap_step!(phase, complete, awaiting);
}
