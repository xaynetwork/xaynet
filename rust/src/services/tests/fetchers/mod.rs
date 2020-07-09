use std::{collections::HashMap, sync::Arc};

use tokio_test::assert_ready;
use tower_test::mock::Spawn;

use crate::{
    crypto::{ByteObject, PublicEncryptKey, PublicSigningKey},
    mask::{seed::EncryptedMaskSeed, Model},
    services::{
        fetchers::{
            MaskLengthRequest,
            MaskLengthService,
            ModelRequest,
            ModelService,
            RoundParamsRequest,
            RoundParamsService,
            ScalarRequest,
            ScalarService,
            SeedDictRequest,
            SeedDictService,
            SumDictRequest,
            SumDictService,
        },
        tests::utils::new_event_channels,
    },
    state_machine::{
        coordinator::{RoundParameters, RoundSeed},
        events::{DictionaryUpdate, MaskLengthUpdate, ModelUpdate, ScalarUpdate},
    },
    SeedDict,
    SumDict,
    UpdateSeedDict,
};

#[tokio::test]
async fn test_mask_length_svc() {
    let (mut publisher, subscriber) = new_event_channels();

    let mut task = Spawn::new(MaskLengthService::new(&subscriber));
    assert_ready!(task.poll_ready()).unwrap();

    let resp = task.call(MaskLengthRequest).await;
    assert_eq!(resp, Ok(None));

    let round_id = subscriber.params_listener().get_latest().round_id;
    publisher.broadcast_mask_length(round_id.clone(), MaskLengthUpdate::New(42));
    assert_ready!(task.poll_ready()).unwrap();
    let resp = task.call(MaskLengthRequest).await;
    assert_eq!(resp, Ok(Some(42)));

    publisher.broadcast_mask_length(round_id, MaskLengthUpdate::Invalidate);
    assert_ready!(task.poll_ready()).unwrap();
    let resp = task.call(MaskLengthRequest).await;
    assert_eq!(resp, Ok(None));
}

#[tokio::test]
async fn test_model_svc() {
    let (mut publisher, subscriber) = new_event_channels();

    let mut task = Spawn::new(ModelService::new(&subscriber));
    assert_ready!(task.poll_ready()).unwrap();

    let resp = task.call(ModelRequest).await;
    assert_eq!(resp, Ok(None));

    let round_id = subscriber.params_listener().get_latest().round_id;
    let model = Arc::new(Model::from(vec![]));
    publisher.broadcast_model(round_id.clone(), ModelUpdate::New(model.clone()));
    assert_ready!(task.poll_ready()).unwrap();
    let resp = task.call(ModelRequest).await;
    assert_eq!(resp, Ok(Some(model)));

    publisher.broadcast_model(round_id, ModelUpdate::Invalidate);
    assert_ready!(task.poll_ready()).unwrap();
    let resp = task.call(ModelRequest).await;
    assert_eq!(resp, Ok(None));
}

#[tokio::test]
async fn test_round_params_svc() {
    let (mut publisher, subscriber) = new_event_channels();
    let initial_params = subscriber.params_listener().get_latest().event;

    let mut task = Spawn::new(RoundParamsService::new(&subscriber));
    assert_ready!(task.poll_ready()).unwrap();

    let resp = task.call(RoundParamsRequest).await;
    assert_eq!(resp, Ok(initial_params));

    let params = RoundParameters {
        pk: PublicEncryptKey::fill_with(0x11),
        sum: 0.42,
        update: 0.42,
        seed: RoundSeed::fill_with(0x11),
    };
    publisher.broadcast_params(params.clone());
    assert_ready!(task.poll_ready()).unwrap();
    let resp = task.call(RoundParamsRequest).await;
    assert_eq!(resp, Ok(params));
}

#[tokio::test]
async fn test_scalar_svc() {
    let (mut publisher, subscriber) = new_event_channels();

    let mut task = Spawn::new(ScalarService::new(&subscriber));
    assert_ready!(task.poll_ready()).unwrap();

    let resp = task.call(ScalarRequest).await;
    assert_eq!(resp, Ok(None));

    let round_id = subscriber.params_listener().get_latest().round_id;
    publisher.broadcast_scalar(round_id.clone(), ScalarUpdate::New(42.42));
    assert_ready!(task.poll_ready()).unwrap();
    let resp = task.call(ScalarRequest).await;
    assert_eq!(resp, Ok(Some(42.42)));

    publisher.broadcast_scalar(round_id, ScalarUpdate::Invalidate);
    assert_ready!(task.poll_ready()).unwrap();
    let resp = task.call(ScalarRequest).await;
    assert_eq!(resp, Ok(None));
}

fn dummy_seed_dict() -> SeedDict {
    let mut dict = HashMap::new();
    dict.insert(PublicSigningKey::fill_with(0xaa), dummy_update_dict());
    dict.insert(PublicSigningKey::fill_with(0xbb), dummy_update_dict());
    dict
}

fn dummy_update_dict() -> UpdateSeedDict {
    let mut dict = HashMap::new();
    dict.insert(
        PublicSigningKey::fill_with(0x11),
        EncryptedMaskSeed::fill_with(0x11),
    );
    dict.insert(
        PublicSigningKey::fill_with(0x22),
        EncryptedMaskSeed::fill_with(0x22),
    );
    dict
}

#[tokio::test]
async fn test_seed_dict_svc() {
    let (mut publisher, subscriber) = new_event_channels();

    let mut task = Spawn::new(SeedDictService::new(&subscriber));
    assert_ready!(task.poll_ready()).unwrap();

    let resp = task.call(SeedDictRequest).await;
    assert_eq!(resp, Ok(None));

    let round_id = subscriber.params_listener().get_latest().round_id;
    let seed_dict = Arc::new(dummy_seed_dict());
    publisher.broadcast_seed_dict(round_id.clone(), DictionaryUpdate::New(seed_dict.clone()));
    assert_ready!(task.poll_ready()).unwrap();
    let resp = task.call(SeedDictRequest).await;
    assert_eq!(resp, Ok(Some(seed_dict)));

    publisher.broadcast_seed_dict(round_id, DictionaryUpdate::Invalidate);
    assert_ready!(task.poll_ready()).unwrap();
    let resp = task.call(SeedDictRequest).await;
    assert_eq!(resp, Ok(None));
}

fn dummy_sum_dict() -> SumDict {
    let mut dict = HashMap::new();
    dict.insert(
        PublicSigningKey::fill_with(0xaa),
        PublicEncryptKey::fill_with(0xcc),
    );
    dict.insert(
        PublicSigningKey::fill_with(0xbb),
        PublicEncryptKey::fill_with(0xdd),
    );
    dict
}

#[tokio::test]
async fn test_sum_dict_svc() {
    let (mut publisher, subscriber) = new_event_channels();

    let mut task = Spawn::new(SumDictService::new(&subscriber));
    assert_ready!(task.poll_ready()).unwrap();

    let resp = task.call(SumDictRequest).await;
    assert_eq!(resp, Ok(None));

    let round_id = subscriber.params_listener().get_latest().round_id;
    let sum_dict = Arc::new(dummy_sum_dict());
    publisher.broadcast_sum_dict(round_id.clone(), DictionaryUpdate::New(sum_dict.clone()));
    assert_ready!(task.poll_ready()).unwrap();
    let resp = task.call(SumDictRequest).await;
    assert_eq!(resp, Ok(Some(sum_dict)));

    publisher.broadcast_sum_dict(round_id, DictionaryUpdate::Invalidate);
    assert_ready!(task.poll_ready()).unwrap();
    let resp = task.call(SumDictRequest).await;
    assert_eq!(resp, Ok(None));
}
