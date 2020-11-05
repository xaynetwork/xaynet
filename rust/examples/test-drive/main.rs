#[macro_use]
extern crate tracing;

#[macro_use]
extern crate async_trait;

use std::{sync::Arc, time::Duration};

use structopt::StructOpt;
use tracing_futures::Instrument;
use tracing_subscriber::*;
use xaynet_core::{
    crypto::SigningKeyPair,
    mask::{BoundType, DataType, FromPrimitives, GroupType, MaskConfig, Model, ModelType},
};
use xaynet_sdk::{client::Client, PetSettings};

mod participant;
mod settings;

#[tokio::main]
async fn main() -> Result<(), std::convert::Infallible> {
    let _fmt_subscriber = FmtSubscriber::builder()
        .with_env_filter(EnvFilter::from_default_env())
        .with_ansi(true)
        .init();

    let opt = settings::Opt::from_args();

    // dummy local model for clients
    let len = opt.len as usize;
    let model = Arc::new(Model::from_primitives(vec![0; len].into_iter()).unwrap());

    let xaynet_client = Client::new(&opt.url, None).unwrap();

    for id in 0..opt.nb_client {
        spawn_participant(id as u32, xaynet_client.clone(), model.clone())
    }

    tokio::signal::ctrl_c().await.unwrap();
    Ok(())
}

fn generate_agent_config() -> PetSettings {
    let mask_config = MaskConfig {
        group_type: GroupType::Prime,
        data_type: DataType::F32,
        bound_type: BoundType::B0,
        model_type: ModelType::M3,
    };
    let keys = SigningKeyPair::generate();
    PetSettings::new(keys, mask_config)
}

fn spawn_participant(id: u32, client: Client, model: Arc<Model>) {
    let config = generate_agent_config();
    let (participant, agent) = participant::Participant::new(config, client, model);
    tokio::spawn(async move {
        participant
            .run()
            .instrument(error_span!("participant", id = id))
            .await;
    });
    tokio::spawn(async move {
        agent
            .run(Duration::from_secs(1))
            .instrument(error_span!("agent", id = id))
            .await;
    });
}
