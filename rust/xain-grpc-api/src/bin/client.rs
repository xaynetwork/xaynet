use log::info;

use xain_grpc_api::logging;

use std::sync::Arc;

use grpcio::{ChannelBuilder, ChannelCredentials, ChannelCredentialsBuilder, EnvBuilder};
use xain_proto::coordinator::RendezvousRequest;
use xain_proto::coordinator_grpc::CoordinatorClient;

fn load_certificates() -> ChannelCredentials {
    // TODO: load certificates dynamically
    let root_cert = std::fs::read_to_string("certs/ca.cer").unwrap();
    let client_cert = std::fs::read_to_string("certs/client.cer").unwrap();
    let client_key = std::fs::read_to_string("certs/client.key").unwrap();

    ChannelCredentialsBuilder::new()
        .root_cert(root_cert.into_bytes())
        .cert(client_cert.into_bytes(), client_key.into_bytes())
        .build()
}

fn main() {
    let channel_credentials = load_certificates();

    let _guard = logging::init_log(None);
    let env = Arc::new(EnvBuilder::new().build());
    let ch = ChannelBuilder::new(env).secure_connect("localhost:50051", channel_credentials);

    let client = CoordinatorClient::new(ch);

    let req = RendezvousRequest::new();
    let reply = client.rendezvous(&req).expect("rpc");
    info!("Client received: {:?}", reply.get_response());
}
