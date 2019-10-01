use xain_proto;

use log::{info,error};

use grpc_api::logging;

use std::io::Read;
use std::sync::Arc;
use std::{io, thread};

use futures::sync::oneshot;
use futures::Future;
use grpcio::{Environment, ServerBuilder, ServerCredentialsBuilder};

use xain_proto::coordinator::{RendezvousRequest, RendezvousReply};
use xain_proto::coordinator_grpc::{self, Coordinator};
use xain_proto::hellonumproto::{NumProtoRequest, NumProtoReply};
use xain_proto::hellonumproto_grpc::{self, NumProtoServer};

#[derive(Clone, Copy)]
struct CoordinatorService;

impl Coordinator for CoordinatorService {
    fn rendezvous(&mut self, ctx: ::grpcio::RpcContext, req: RendezvousRequest, sink: ::grpcio::UnarySink<RendezvousReply>) {
        println!("Incoming request: {:?}", req);
        let reply = RendezvousReply::default();
        let f = sink
            .success(reply)
            .map_err(move |e| error!("failed to reply {:?}: {:?}", req, e));
        ctx.spawn(f)
    }
}

#[derive(Clone, Copy)]
struct NumProtoService;

impl NumProtoServer for NumProtoService {
    fn say_hello_num_proto(&mut self, _ctx: ::grpcio::RpcContext, _req: NumProtoRequest, _sink: ::grpcio::UnarySink<NumProtoReply>) {
        unimplemented!();
    }
}

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
    
    let coordinator_service =
        coordinator_grpc::create_coordinator(CoordinatorService);
    let numproto_service =
        hellonumproto_grpc::create_num_proto_server(NumProtoService);

    let mut server = ServerBuilder::new(env)
        .register_service(coordinator_service)
        .register_service(numproto_service)
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
