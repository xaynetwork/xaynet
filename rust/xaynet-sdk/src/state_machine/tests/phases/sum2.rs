use mockall::Sequence;
use xaynet_core::{
    crypto::{ByteObject, EncryptKeyPair, EncryptKeySeed, PublicEncryptKey},
    mask::{FromPrimitives, MaskConfigPair, MaskObject, MaskSeed, Masker, Model},
    UpdateSeedDict,
};

use crate::{
    state_machine::{
        tests::utils::{shared_state, SelectFor, SigningKeyGenerator},
        Awaiting,
        IntoPhase,
        MockIO,
        Phase,
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

fn make_sum2(shared: &SharedState) -> Sum2 {
    let ephm_keys = EncryptKeyPair::derive_from_seed(&EncryptKeySeed::zeroed());
    let sk = &shared.keys.secret;
    let seed = shared.round_params.seed.as_slice();
    let signature = sk.sign_detached(&[seed, b"sum"].concat());
    Sum2 {
        ephm_keys,
        sum_signature: signature,
        seed_dict: None,
        seeds: None,
        mask: None,
        mask_length: None,
        message: None,
    }
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
    let scalar = 1.0;
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

async fn step2_get_mask_length(mut phase: Phase<Sum2>) -> Phase<Sum2> {
    phase.with_io_mock(move |mock| {
        mock.expect_get_mask_length()
            .times(1)
            .returning(move || Ok(Some(4)));
    });
    let phase = unwrap_step!(phase, complete, sum2);

    // Calling `fetch_mask_length` again should return Progress::Continue
    let mut phase = unwrap_progress_continue!(phase, fetch_mask_length, async);
    phase.check_io_mock();
    phase
}

async fn step3_decrypt_seeds(phase: Phase<Sum2>) -> Phase<Sum2> {
    let phase = unwrap_step!(phase, complete, sum2);
    assert!(phase.state.private.seeds.is_some());
    // Make sure this steps consumes the seed dict.
    assert!(phase.state.private.seed_dict.is_none());
    phase
}

async fn step4_aggregate_masks(phase: Phase<Sum2>) -> Phase<Sum2> {
    let phase = unwrap_step!(phase, complete, sum2);
    assert!(phase.state.private.mask.is_some());
    // Make sure this steps consumes the seeds.
    assert!(phase.state.private.seeds.is_none());
    phase
}

async fn step5_compose_sum2_message(phase: Phase<Sum2>) -> Phase<Sum2> {
    let phase = unwrap_step!(phase, complete, sum2);
    assert!(phase.state.private.message.is_some());
    // Make sure this steps consumes the mask.
    assert!(phase.state.private.seeds.is_none());
    phase
}

async fn step6_send_message(mut phase: Phase<Sum2>) -> Phase<Awaiting> {
    phase.with_io_mock(|mock| {
        let mut seq = Sequence::new();
        mock.expect_send_message()
            .times(1)
            .in_sequence(&mut seq)
            .returning(|_| Ok(()));
        mock.expect_notify_idle()
            .times(1)
            .in_sequence(&mut seq)
            .return_const(());
    });
    let mut phase = unwrap_step!(phase, complete, awaiting);
    phase.check_io_mock();
    phase
}

#[tokio::test]
async fn test_phase() {
    let phase = make_phase();
    let phase = step1_fetch_seed_dict(phase).await;
    let phase = step2_get_mask_length(phase).await;
    let phase = step3_decrypt_seeds(phase).await;
    let phase = step4_aggregate_masks(phase).await;
    let phase = step5_compose_sum2_message(phase).await;
    let _phase = step6_send_message(phase).await;
}
