pub mod utils;

use crate::storage::{coordinator_storage::redis, model_storage, Storage, Store};

pub async fn init_store() -> impl Storage {
    let coordinator_store = redis::tests::init_client().await;

    let model_store = {
        #[cfg(not(feature = "model-persistence"))]
        {
            model_storage::noop::NoOp
        }

        #[cfg(feature = "model-persistence")]
        {
            model_storage::s3::tests::init_client().await
        }
    };

    Store::new(coordinator_store, model_store)
}
