use crate::{
    settings::RedisSettings,
    storage::{coordinator_storage::redis, Storage, Store},
};

pub async fn init(
    redis_settings: RedisSettings,
    #[cfg(feature = "model-persistence")] s3_settings: S3Settings,
) -> impl Storage {
    tracing::debug!("initialize");
    let coordinator_store = loop {
        match redis::Client::new(redis_settings.url.clone()).await {
            Ok(coordinator_store) => break coordinator_store,
            Err(err) => {
                tracing::warn!("{}", err);
                tokio::time::delay_for(tokio::time::Duration::from_secs(5)).await
            }
        }
    };

    let model_store = {
        #[cfg(not(feature = "model-persistence"))]
        {
            crate::storage::model_storage::noop::NoOp
        }

        #[cfg(feature = "model-persistence")]
        {
            let s3 = s3::Client::new(s3_settings).expect("failed to create S3 client");
            s3.create_global_models_bucket()
                .await
                .expect("failed to create bucket for global models");
            s3
        }
    };

    Store::new(coordinator_store, model_store)
}
