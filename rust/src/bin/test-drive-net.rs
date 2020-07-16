#[macro_use]
extern crate tracing;

use rand::Rng;
use structopt::StructOpt;
use tokio::signal;
use tracing_subscriber::*;
use xain_fl::{
    client::{AsyncClient, ClientError},
    mask::{FromPrimitives, Model},
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
    #[structopt(
        default_value = "1",
        short,
        help = "The time period at which to poll for service data, in seconds"
    )]
    period: u64,
    #[structopt(default_value = "10", short, help = "The number of clients")]
    nb_client: u32,
}

/// Test-drive script of a (local, but networked) federated
/// learning session, intended for use as a mini integration test.
/// It assumes that a coordinator is already running.
#[tokio::main]
async fn main() -> Result<(), ClientError> {
    let mut rng = rand::thread_rng();
    let _fmt_subscriber = FmtSubscriber::builder()
        .with_env_filter(EnvFilter::from_default_env())
        .with_ansi(true)
        .init();

    let opt = Opt::from_args();

    // dummy local model for clients
    let len = opt.len as usize;

    let mut clients = Vec::with_capacity(opt.nb_client as usize);
    let (shutdown_tx, _) = tokio::sync::broadcast::channel(1);
    for id in 0..opt.nb_client {
        let mut client = AsyncClient::new(&opt.url)?;
        let client_fut = client.start(shutdown_tx.subscribe());
        tokio::spawn(async move { client_fut.await });
        clients.push(client);
    }

    for _ in 0..opt.nb_client {
        for client in clients.iter_mut() {
            let model =
                Model::from_primitives(vec![rng.gen_range(0, 10); len].into_iter()).unwrap();
            client.set_local_model(model)
        }
    }

    signal::ctrl_c().await;

    //wait for all the clients to finish
    drop(shutdown_tx);
    Ok(())
}
