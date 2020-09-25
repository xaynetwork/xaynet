use std::path::PathBuf;

#[macro_use]
extern crate tracing;

use structopt::StructOpt;
use tokio::signal;
use tracing_subscriber::*;

use xaynet_client::{
    api::{HttpApiClient, HttpApiClientError},
    Client,
    ClientError,
};
use xaynet_core::mask::{FromPrimitives, Model};

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

    #[structopt(
        default_value = "1",
        short,
        help = "The time period at which to poll for service data, in seconds"
    )]
    period: u64,

    #[structopt(default_value = "10", short, help = "The number of clients")]
    nb_client: u32,

    #[structopt(
        short,
        long,
        parse(from_os_str),
        help = "The list of trusted DER and PEM encoded TLS server certificates"
    )]
    certificates: Option<Vec<PathBuf>>,
}

/// Test-drive script of a (local, but networked) federated
/// learning session, intended for use as a mini integration test.
/// It assumes that a coordinator is already running.
#[tokio::main]
async fn main() -> Result<(), ClientError<HttpApiClientError>> {
    let _fmt_subscriber = FmtSubscriber::builder()
        .with_env_filter(EnvFilter::from_default_env())
        .with_ansi(true)
        .init();

    let opt = Opt::from_args();

    // dummy local model for clients
    let len = opt.len as usize;
    let model = Model::from_primitives(vec![0; len].into_iter()).unwrap();

    let certificates =
        HttpApiClient::certificates_from(&opt.certificates).map_err(ClientError::Api)?;
    let mut clients = Vec::with_capacity(opt.nb_client as usize);
    for id in 0..opt.nb_client {
        let mut client = Client::new(
            opt.period,
            id,
            HttpApiClient::new(&opt.url, certificates.clone()).map_err(ClientError::Api)?,
        )?;
        client.local_model = Some(model.clone());
        let join_hdl = tokio::spawn(async move {
            tokio::select! {
                _ = signal::ctrl_c() => {}
                result = client.start() => {
                    error!("{:?}", result);
                }
            }
        });
        clients.push(join_hdl);
    }

    // wait for all the clients to finish
    for client in clients {
        let _ = client.await;
    }

    Ok(())
}
