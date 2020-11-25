use std::{fs::File, io::Read, sync::Arc, time::Duration};

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
        spawn_participant(id as u32, &opt, model.clone())?;
    }

    tokio::signal::ctrl_c().await.unwrap();
    Ok(())
}

fn generate_agent_config() -> PetSettings {
    let keys = SigningKeyPair::generate();
    PetSettings::new(keys)
}

fn build_http_client(settings: &settings::Opt) -> reqwest::Client {
    let builder = reqwest::ClientBuilder::new();

    let builder = if let Some(ref path) = settings.certificate {
        let mut buf = Vec::new();
        File::open(path).unwrap().read_to_end(&mut buf).unwrap();
        let root_cert = reqwest::Certificate::from_pem(&buf).unwrap();
        builder.use_rustls_tls().add_root_certificate(root_cert)
    } else {
        builder
    };

    let builder = if let Some(ref path) = settings.identity {
        let mut buf = Vec::new();
        File::open(path).unwrap().read_to_end(&mut buf).unwrap();
        let identity = reqwest::Identity::from_pem(&buf).unwrap();
        builder.use_rustls_tls().identity(identity)
    } else {
        builder
    };

    builder.build().unwrap()
}

fn spawn_participant(
    id: u32,
    settings: &settings::Opt,
    model: Arc<Model>,
) -> Result<(), ClientError> {
    let config = generate_agent_config();
    let http_client = build_http_client(settings);
    let client = Client::new(http_client, &settings.url).unwrap();

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
