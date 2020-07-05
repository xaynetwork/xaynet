use futures::stream::{FuturesUnordered, StreamExt};
use structopt::StructOpt;
use tokio::{signal, task::JoinHandle};
use tracing_subscriber::*;
use xain_fl::{
    client::{Client, ClientError, Task},
    mask::{FromPrimitives, Model},
};

#[macro_use]
extern crate tracing;

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
    no: u32,
}

/// Test-drive script of a (local, but networked) single-round federated
/// learning session, intended for use as a mini integration test. It assumes
/// that a [`Service`] is already running and listening to
/// http://127.0.0.1:8081.
///
/// 10 [`Client`]s are spawned on the tokio event loop. This serves as a simple
/// example of getting started with the project, and may later be the basis for
/// more automated tests.
#[tokio::main]
async fn main() -> Result<(), ClientError> {
    let _fmt_subscriber = FmtSubscriber::builder()
        .with_env_filter(EnvFilter::from_default_env())
        .with_ansi(true)
        .init();

    let opt = Opt::from_args();

    // // dummy local model for clients
    let len = opt.len as usize;
    let model = Model::from_primitives(vec![0; len].into_iter()).unwrap();

    let mut clients = FuturesUnordered::<JoinHandle<()>>::new();
    for id in 0..opt.no {
        let mut client = Client::new_with_addr(opt.period, id, &opt.url)?;
        client.local_model = Some(model.clone());
        let join_hdl = tokio::spawn(async move {
            tokio::select! {
                _ = signal::ctrl_c() => {
                    info!("ctrl-c received!");
                }
                result = client.start() => {
                    error!("{:?}", result);
                }
            }
        });
        clients.push(join_hdl);
    }

    // wait for all the clients to finish
    loop {
        if clients.next().await.is_none() {
            break;
        }
    }

    Ok(())
}
