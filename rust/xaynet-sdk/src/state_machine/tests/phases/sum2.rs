use mockall::Sequence;
use xaynet_core::{
    crypto::{ByteObject, EncryptKeyPair, EncryptKeySeed, PublicEncryptKey},
    mask::{FromPrimitives, MaskConfigPair, MaskObject, MaskSeed, Masker, Model, Scalar},
    UpdateSeedDict,
};

use crate::{
    state_machine::{
        tests::utils::{shared_state, SelectFor, SigningKeyGenerator},
        IntoPhase,
        MockIO,
        Phase,
        SendingSum2,
        SharedState,
        State,
        Sum2,
    },
    unwrap_progress_continue,
    unwrap_step,
};

/// Instantiate a sum phase.
fn make_phase() -> Phase<Sum2> {
    let shared = shared_state(SelectFor::Sum);
    let sum2 = make_sum2(&shared);

    // Check IntoPhase<Sum2> implementation
    let mock = MockIO::new();
    let mut phase: Phase<Sum2> = State::new(shared, sum2).into_phase(Box::new(mock));

    phase.check_io_mock();
    phase
}

fn make_sum2(shared: &SharedState) -> Box<Sum2> {
    let ephm_keys = EncryptKeyPair::derive_from_seed(&EncryptKeySeed::zeroed());
    let sk = &shared.keys.secret;
    let seed = shared.round_params.seed.as_slice();
    let signature = sk.sign_detached(&[seed, b"sum"].concat());
    Box::new(Sum2 {
        ephm_keys,
        sum_signature: signature,
        seed_dict: None,
        seeds: None,
        mask: None,
    })
}

fn make_seed_dict(mask_config: MaskConfigPair, ephm_pk: PublicEncryptKey) -> UpdateSeedDict {
    let (seed, _mask) = make_masked_model(mask_config);
    let mut key_gen = SigningKeyGenerator::new();
    let mut dict = UpdateSeedDict::new();
    for _ in 0..4 {
        let pk = key_gen.next().public;
        dict.insert(pk, seed.encrypt(&ephm_pk));
    }
    dict
}

fn make_model() -> Model {
    Model::from_primitives(vec![1.0, 2.0, 3.0, 4.0].into_iter()).unwrap()
}

fn make_masked_model(mask_config: MaskConfigPair) -> (MaskSeed, MaskObject) {
    let masker = Masker::new(mask_config);
    let scalar = Scalar::unit();
    let model = make_model();
    masker.mask(scalar, &model)
}

async fn step1_fetch_seed_dict(mut phase: Phase<Sum2>) -> Phase<Sum2> {
    let mask_config = phase.state.shared.round_params.mask_config;
    let ephm_pk = phase.state.private.ephm_keys.public;
    phase.with_io_mock(move |mock| {
        let mut seq = Sequence::new();
        // The first time the state machine fetches the seed dict,
        // pretend it's not published yet
        mock.expect_get_seeds()
            .times(1)
            .in_sequence(&mut seq)
            .returning(|_| Ok(None));
        // The second time, return it
        mock.expect_get_seeds()
            .times(1)
            .in_sequence(&mut seq)
            .returning(move |_| Ok(Some(make_seed_dict(mask_config, ephm_pk))));
    });

    // First time: no progress should be made, since we didn't
    // fetch any seed dict yet
    let phase = unwrap_step!(phase, pending, sum2);

    // Second time: now the state machine should have made progress
    let phase = unwrap_step!(phase, complete, sum2);

    // Calling `fetch_seed_dict` again should return Progress::Continue
    let mut phase = unwrap_progress_continue!(phase, fetch_seed_dict, async);
    phase.check_io_mock();
    phase
}

async fn step2_decrypt_seeds(phase: Phase<Sum2>) -> Phase<Sum2> {
    let phase = unwrap_step!(phase, complete, sum2);
    assert!(phase.state.private.seeds.is_some());
    // Make sure this steps consumes the seed dict.
    assert!(phase.state.private.seed_dict.is_none());
    phase
}

async fn step3_aggregate_masks(phase: Phase<Sum2>) -> Phase<Sum2> {
    let phase = unwrap_step!(phase, complete, sum2);
    assert!(phase.state.private.mask.is_some());
    // Make sure this steps consumes the seeds.
    assert!(phase.state.private.seeds.is_none());
    phase
}

async fn step4_into_sending_phase(phase: Phase<Sum2>) -> Phase<SendingSum2> {
    let phase = unwrap_step!(phase, complete, sending_sum2);
    phase
}

#[tokio::test]
async fn test_phase() {
    let phase = make_phase();
    let phase = step1_fetch_seed_dict(phase).await;
    let phase = step2_decrypt_seeds(phase).await;
    let phase = step3_aggregate_masks(phase).await;
    let _phase = step4_into_sending_phase(phase).await;
}
