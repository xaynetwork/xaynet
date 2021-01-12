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

fn make_sum(shared: &SharedState) -> Box<Sum> {
    let ephm_keys = EncryptKeyPair::derive_from_seed(&EncryptKeySeed::zeroed());
    let sk = &shared.keys.secret;
    let seed = shared.round_params.seed.as_slice();
    let signature = sk.sign_detached(&[seed, b"sum"].concat());
    Box::new(Sum {
        ephm_keys,
        sum_signature: signature,
    })
}

#[tokio::test]
async fn test_phase() {
    let io = MockIO::new();
    let phase = make_phase(io);
    let _phase = unwrap_step!(phase, complete, sending_sum);
}

#[derive(Error, Debug)]
#[error("error")]
struct DummyErr;
