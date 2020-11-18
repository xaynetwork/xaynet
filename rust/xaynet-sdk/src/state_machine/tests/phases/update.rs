use mockall::Sequence;
use xaynet_core::{
    crypto::ByteObject,
    mask::{FromPrimitives, Model},
    SumDict,
};

use crate::{
    save_and_restore,
    state_machine::{
        tests::utils::{shared_state, EncryptKeyGenerator, SelectFor, SigningKeyGenerator},
        Awaiting,
        IntoPhase,
        MockIO,
        Phase,
        SharedState,
        State,
        Update,
    },
    unwrap_progress_continue,
    unwrap_step,
};

/// Instantiate a sum phase.
fn make_phase() -> Phase<Update> {
    let shared = shared_state(SelectFor::Sum);
    let update = make_update(&shared);

    // Check IntoPhase<Update> implementation
    let mut mock = MockIO::new();
    mock.expect_notify_update().times(1).return_const(());
    mock.expect_notify_load_model().times(1).return_const(());
    let mut phase: Phase<Update> = State::new(shared, update).into_phase(Box::new(mock));

    phase.check_io_mock();
    phase
}

fn make_update(shared: &SharedState) -> Update {
    let sk = &shared.keys.secret;
    let seed = shared.round_params.seed.as_slice();
    let sum_signature = sk.sign_detached(&[seed, b"sum"].concat());
    let update_signature = sk.sign_detached(&[seed, b"update"].concat());
    Update {
        sum_signature,
        update_signature,
        sum_dict: None,
        seed_dict: None,
        model: None,
        mask: None,
        message: None,
    }
}

fn make_model() -> Model {
    let weights: Vec<f32> = vec![1.1, 2.2, 3.3, 4.4];
    Model::from_primitives(weights.into_iter()).unwrap()
}

fn make_sum_dict() -> SumDict {
    let mut dict = SumDict::new();

    let mut signing_keys = SigningKeyGenerator::new();
    let mut encrypt_keys = EncryptKeyGenerator::new();

    dict.insert(signing_keys.next().public, encrypt_keys.next().public);
    dict.insert(signing_keys.next().public, encrypt_keys.next().public);

    dict
}

async fn step1_fetch_sum_dict(mut phase: Phase<Update>) -> Phase<Update> {
    phase.with_io_mock(|mock| {
        let mut seq = Sequence::new();
        // The first time the state machine fetches the sum dict,
        // pretend it's not publiches yet
        mock.expect_get_sums()
            .times(1)
            .in_sequence(&mut seq)
            .returning(|| Ok(None));
        // The second time, return a sum dictionary.
        mock.expect_get_sums()
            .times(1)
            .in_sequence(&mut seq)
            .returning(|| Ok(Some(make_sum_dict())));
    });

    // First time: no progress should be made, since we didn't
    // fetch any sum dict yet
    let phase = unwrap_step!(phase, pending, update);

    // Second time: now the state machine should have made progress
    let phase = unwrap_step!(phase, complete, update);

    // Calling `fetch_sum_dict` again should return Progress::Continue
    let mut phase = unwrap_progress_continue!(phase, fetch_sum_dict, async);
    phase.check_io_mock();
    phase
}

async fn step2_load_model(mut phase: Phase<Update>) -> Phase<Update> {
    phase.with_io_mock(|mock| {
        let mut seq = Sequence::new();
        // The first time the state machine fetches the sum dict,
        // pretend it's not published yet
        mock.expect_load_model()
            .times(1)
            .in_sequence(&mut seq)
            .returning(|| Ok(None));
        // The second time, return a sum dictionary.
        mock.expect_load_model()
            .times(1)
            .in_sequence(&mut seq)
            .returning(|| Ok(Some(Box::new(make_model()))));
    });

    // First time: no progress should be made, since we didn't
    // load any model
    let phase = unwrap_step!(phase, pending, update);

    // Second time: now the state machine should have made progress
    let phase = unwrap_step!(phase, complete, update);

    // Calling `load_model` again should return Progress::Continue
    let mut phase = unwrap_progress_continue!(phase, load_model, async);
    phase.check_io_mock();
    phase
}

async fn step3_mask_model(phase: Phase<Update>) -> Phase<Update> {
    let phase = unwrap_step!(phase, complete, update);
    let mut phase = unwrap_progress_continue!(phase, mask_model);
    phase.check_io_mock();
    phase
}

async fn step4_build_seed_dict(phase: Phase<Update>) -> Phase<Update> {
    let phase = unwrap_step!(phase, complete, update);
    let mut phase = unwrap_progress_continue!(phase, build_seed_dict);
    phase.check_io_mock();
    phase
}

async fn step5_compose_update_message(phase: Phase<Update>) -> Phase<Update> {
    let phase = unwrap_step!(phase, complete, update);
    let mut phase = unwrap_progress_continue!(phase, compose_update_message);
    phase.check_io_mock();
    phase
}

async fn step6_send_message(mut phase: Phase<Update>) -> Phase<Awaiting> {
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
async fn test_update_phase() {
    let phase = make_phase();
    let phase = step1_fetch_sum_dict(phase).await;
    let phase = step2_load_model(phase).await;
    let phase = step3_mask_model(phase).await;
    let phase = step4_build_seed_dict(phase).await;
    let phase = step5_compose_update_message(phase).await;
    step6_send_message(phase).await;
}

#[tokio::test]
async fn test_save_and_restore() {
    let phase = make_phase();
    let mut phase = step1_fetch_sum_dict(phase).await;

    phase.with_io_mock(|mock| {
        let mut seq = Sequence::new();
        mock.expect_notify_update()
            .times(1)
            .in_sequence(&mut seq)
            .return_const(());
        mock.expect_notify_load_model()
            .times(1)
            .in_sequence(&mut seq)
            .return_const(());
    });
    let phase = save_and_restore!(phase, Update);

    let mut phase = step2_load_model(phase).await;
    phase.with_io_mock(|mock| {
        mock.expect_notify_update().times(1).return_const(());
    });
    let phase = save_and_restore!(phase, Update);

    let phase = step3_mask_model(phase).await;
    let phase = step4_build_seed_dict(phase).await;
    let mut phase = step5_compose_update_message(phase).await;
    phase.with_io_mock(|mock| {
        mock.expect_notify_update().times(1).return_const(());
    });
    save_and_restore!(phase, Update);
}
