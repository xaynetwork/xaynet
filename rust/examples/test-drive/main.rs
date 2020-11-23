use std::{sync::Arc, time::Duration};

use structopt::StructOpt;
use tracing::error_span;
use tracing_futures::Instrument;
use tracing_subscriber::{EnvFilter, FmtSubscriber};

use xaynet_core::{
    crypto::SigningKeyPair,
    mask::{FromPrimitives, Model},
};
use xaynet_sdk::{
    client::{Client, ClientError},
    settings::PetSettings,
};

mod participant;
mod settings;

#[tokio::main]
async fn main() -> Result<(), ClientError> {
    let _fmt_subscriber = FmtSubscriber::builder()
        .with_env_filter(EnvFilter::from_default_env())
        .with_ansi(true)
        .init();

    let opt = settings::Opt::from_args();

    // dummy local model for clients
    let len = opt.len as usize;
    let model = Arc::new(Model::from_primitives(vec![0; len].into_iter()).unwrap());

    for id in 0..opt.nb_client {
        spawn_participant(id as u32, &opt.url, model.clone())?;
    }

    tokio::signal::ctrl_c().await.unwrap();
    Ok(())
}

fn generate_agent_config() -> PetSettings {
    let keys = SigningKeyPair::generate();
    PetSettings::new(keys)
}

fn spawn_participant(id: u32, url: &str, model: Arc<Model>) -> Result<(), ClientError> {
    let config = generate_agent_config();
    let http_client = reqwest::ClientBuilder::new().build().unwrap();
    let client = Client::new(http_client, url).unwrap();

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
    Ok(())
}
