use std::fs;
use std::sync::Arc;

use clap::{App, Arg};
use grpcio::{ChannelBuilder, ChannelCredentialsBuilder, EnvBuilder};

use xain_grpc::proto::coordinator::RendezvousRequest;
use xain_grpc::proto::coordinator_grpc::CoordinatorClient;

type DynError = Box<dyn std::error::Error + Send + Sync>;

fn main() -> Result<(), DynError> {
    // Set up logging.
    env_logger::Builder::new().filter_level(log::LevelFilter::Info).init();
    grpcio::redirect_log();

    // Parse arguments passed to the program.
    let args = App::new("xain-coordinator-client")
        .version("0.1")
        .about("A coordinator client for testing the XAIN distributed ML framework!")
        .author("The XAIN developers")
        .arg(Arg::with_name("root-cert").short("r").required(true).takes_value(true))
        .arg(Arg::with_name("client-cert").short("s").required(true).takes_value(true))
        .arg(Arg::with_name("client-key").short("k").required(true).takes_value(true))
        .get_matches();

    // Load certificates.
    let root_cert = fs::read_to_string(args.value_of("root-cert").unwrap())?;
    let client_cert = fs::read_to_string(args.value_of("client-cert").unwrap())?;
    let client_key = fs::read_to_string(args.value_of("client-key").unwrap())?;
    let credentials = ChannelCredentialsBuilder::new()
        .root_cert(root_cert.into_bytes())
        .cert(client_cert.into_bytes(), client_key.into_bytes())
        .build();

    // Create gRPC event loop.
    let env = Arc::new(EnvBuilder::new().build());

    // Start Coordinator client.
    let channel = ChannelBuilder::new(env).secure_connect("localhost:50051", credentials);
    let client = CoordinatorClient::new(channel);

    // Send a rendezvous request and wait for the reply.
    let req = RendezvousRequest::new();
    let reply = client.rendezvous(&req)?;
    println!("Client sent rendezvous");
    println!("Client received: {:?}", reply.get_response());

    Ok(())
}
