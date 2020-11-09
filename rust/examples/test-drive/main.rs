use std::{path::PathBuf, sync::Arc, time::Duration};

use reqwest::Certificate;
use structopt::StructOpt;
use tracing::error_span;
use tracing_futures::Instrument;
use tracing_subscriber::{EnvFilter, FmtSubscriber};

use xaynet_core::{
    crypto::SigningKeyPair,
    mask::{BoundType, DataType, FromPrimitives, GroupType, MaskConfig, Model, ModelType},
};
use xaynet_sdk::{
    client::{Client, ClientError},
    PetSettings,
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

    // optional certificates for TLS server authentication
    let certificates = opt
        .certificates
        .as_ref()
        .map(Client::certificates_from)
        .transpose()?;

    for id in 0..opt.nb_client {
        spawn_participant(
            id as u32,
            &opt.url,
            certificates.clone(),
            &opt.identity,
            model.clone(),
        )?;
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

fn spawn_participant(
    id: u32,
    url: &str,
    certificates: Option<Vec<Certificate>>,
    identity: &Option<PathBuf>,
    model: Arc<Model>,
) -> Result<(), ClientError> {
    // optional identity for TLS client authentication (`Identity` doesn't implement `Clone` because
    // every participant should of course use its own identity in a real use case, therefore we have
    // to create it here for every client again)
    let identity = identity.as_ref().map(Client::identity_from).transpose()?;

    let config = generate_agent_config();
    let client = Client::new(url, certificates, identity).unwrap();

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
