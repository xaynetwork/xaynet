use log::info;

use grpc_api::logging;
use grpc_api::{CoordinatorService, NumProtoService};

use std::io::Read;
use std::sync::Arc;
use std::{io, thread};

use futures::sync::oneshot;
use futures::Future;
use grpcio::{Environment, ServerBuilder, ServerCredentialsBuilder};

fn main() {
    let root_cert = std::fs::read_to_string("certs/ca.cer").unwrap();
    let server_cert = std::fs::read_to_string("certs/server.cer").unwrap();
    let private_key = std::fs::read_to_string("certs/server.key").unwrap();

    let server_credentials = ServerCredentialsBuilder::new()
        .root_cert(root_cert.into_bytes(), true)
        .add_cert(server_cert.into_bytes(), private_key.into_bytes())
        .build();

    let _guard = logging::init_log(None);
    let env = Arc::new(Environment::new(1));

    let mut server = ServerBuilder::new(env)
        .register_service(CoordinatorService::create())
        .register_service(NumProtoService::create())
        .bind_secure("127.0.0.1", 50_051, server_credentials)
        .build()
        .unwrap();

    server.start();

    for &(ref host, port) in server.bind_addrs() {
        info!("listening on {}:{}", host, port);
    }

    let (tx, rx) = oneshot::channel();
    thread::spawn(move || {
        info!("Press ENTER to exit...");
        let _ = io::stdin().read(&mut [0]).unwrap();
        tx.send(())
    });
    let _ = rx.wait();
    let _ = server.shutdown().wait();
}
