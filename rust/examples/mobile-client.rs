extern crate tracing;

use std::io::{stdin, stdout, Read, Write};
use structopt::StructOpt;
use tracing_subscriber::*;
use xaynet::{
    certificate::Certificate,
    client::mobile_client::{participant::ParticipantSettings, MobileClient},
    crypto::SigningKeyPair,
    mask::{BoundType, DataType, FromPrimitives, GroupType, MaskConfig, Model, ModelType},
};

#[derive(Debug, StructOpt)]
#[structopt(name = "Test Drive")]
struct Opt {
    #[structopt(
        default_value = "http://127.0.0.1:8081",
        short,
        help = "The URL of the coordinator"
    )]
    url: String,
    #[structopt(default_value = "4", short, help = "The length of the model")]
    len: u32,
}

fn pause() {
    let mut stdout = stdout();
    stdout.write_all(b"Press Enter to continue...").unwrap();
    stdout.flush().unwrap();
    stdin().read_exact(&mut [0]).unwrap();
}

fn get_participant_settings() -> ParticipantSettings {
    sodiumoxide::init().unwrap();

    let SigningKeyPair { public: _, secret } = SigningKeyPair::generate();
    ParticipantSettings {
        secret_key: secret,
        mask_config: MaskConfig {
            group_type: GroupType::Prime,
            data_type: DataType::F32,
            bound_type: BoundType::B0,
            model_type: ModelType::M3,
        },
        certificate: Certificate::new(),
    }
}

fn main() -> Result<(), ()> {
    let opt = Opt::from_args();

    let _fmt_subscriber = FmtSubscriber::builder()
        .with_env_filter(EnvFilter::from_default_env())
        .with_ansi(true)
        .init();

    let mut client = MobileClient::new(&opt.url, get_participant_settings());
    let model = Model::from_primitives(vec![1; opt.len as usize].into_iter()).unwrap();

    loop {
        client.set_local_model(model.clone());
        client.next();
        pause();
    }
}
