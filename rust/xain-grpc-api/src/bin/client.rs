use log::info;

use xain_grpc_api::logging;

use std::sync::Arc;

use grpcio::{ChannelBuilder, ChannelCredentials, ChannelCredentialsBuilder, EnvBuilder};
use xain_proto::coordinator::RendezvousRequest;
use xain_proto::coordinator_grpc::CoordinatorClient;

use clap::{App, Arg};

type AppError = Box<dyn std::error::Error + Send + Sync>;

fn load_certificates(args: &clap::ArgMatches) -> Result<ChannelCredentials, AppError> {
    // TODO: load certificates dynamically
    let root_cert = std::fs::read_to_string(args.value_of("root-cert").unwrap())?;
    let client_cert = std::fs::read_to_string(args.value_of("client-cert").unwrap())?;
    let client_key = std::fs::read_to_string(args.value_of("client-key").unwrap())?;

    Ok(ChannelCredentialsBuilder::new()
        .root_cert(root_cert.into_bytes())
        .cert(client_cert.into_bytes(), client_key.into_bytes())
        .build())
}

fn app() -> App<'static, 'static> {
    App::new("xain-coordinator-client")
        .version("0.1")
        .about("A coordinator client for testing the XAIN distributed ML framework!")
        .author("The XAIN developers")
        .arg(Arg::with_name("root-cert")
            .short("r")
            .required(true)
            .takes_value(true))
        .arg(Arg::with_name("client-cert")
            .short("s")
            .required(true)
            .takes_value(true))
        .arg(Arg::with_name("client-key")
            .short("k")
            .required(true)
            .takes_value(true))
}

fn main() -> Result<(), AppError> {
    let args = app().get_matches();

    let channel_credentials = load_certificates(&args)?;

    let _guard = logging::init_log(None);
    let env = Arc::new(EnvBuilder::new().build());
    let ch = ChannelBuilder::new(env).secure_connect("localhost:50051", channel_credentials);

    let client = CoordinatorClient::new(ch);

    let req = RendezvousRequest::new();
    let reply = client.rendezvous(&req).expect("rpc");
    info!("Client received: {:?}", reply.get_response());

    Ok(())
}
