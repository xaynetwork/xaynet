use std::io::Read;
use std::sync::Arc;
use std::{io, thread};

use clap::{App, Arg};
use futures::future::Future;
use futures::sync::oneshot;
use grpcio::{Environment, ServerBuilder, ServerCredentials, ServerCredentialsBuilder};
use log::error;
use log::info;

use xain_grpc::logging;
use xain_grpc::proto::coordinator::{
    HeartbeatReply, HeartbeatRequest, RendezvousReply, RendezvousRequest,
};
use xain_grpc::proto::coordinator_grpc::{self, Coordinator};

type ServerError = Box<dyn std::error::Error + Send + Sync>;

fn load_certificates(args: &clap::ArgMatches) -> Result<ServerCredentials, ServerError> {
    let root_cert = std::fs::read_to_string(args.value_of("root-cert").unwrap())?;
    let server_cert = std::fs::read_to_string(args.value_of("server-cert").unwrap())?;
    let private_key = std::fs::read_to_string(args.value_of("server-key").unwrap())?;

    Ok(ServerCredentialsBuilder::new()
        .root_cert(root_cert.into_bytes(), true)
        .add_cert(server_cert.into_bytes(), private_key.into_bytes())
        .build())
}

fn app() -> App<'static, 'static> {
    App::new("xain-coordinator")
        .version("0.1")
        .about("The coordinator for the XAIN distributed ML framework!")
        .author("The XAIN developers")
        .arg(Arg::with_name("root-cert").short("r").required(true).takes_value(true))
        .arg(Arg::with_name("server-cert").short("s").required(true).takes_value(true))
        .arg(Arg::with_name("server-key").short("k").required(true).takes_value(true))
}

#[derive(Clone)]
pub struct CoordinatorService;

impl CoordinatorService {
    pub fn create() -> grpcio::Service {
        coordinator_grpc::create_coordinator(CoordinatorService)
    }
}

impl Coordinator for CoordinatorService {
    fn rendezvous(
        &mut self,
        ctx: grpcio::RpcContext,
        req: RendezvousRequest,
        sink: grpcio::UnarySink<RendezvousReply>,
    ) {
        println!("Incoming request: {:?}", req);
        let reply = RendezvousReply::default();
        let f = sink.success(reply).map_err(move |e| error!("failed to reply {:?}: {:?}", req, e));
        ctx.spawn(f)
    }

    fn heartbeat(
        &mut self,
        _ctx: grpcio::RpcContext,
        _req: HeartbeatRequest,
        _sink: grpcio::UnarySink<HeartbeatReply>,
    ) {
        unimplemented!()
    }
}

fn main() -> Result<(), ServerError> {
    let args = app().get_matches();

    let server_credentials = load_certificates(&args)?;

    let _guard = logging::init_log(None);

    let env = Arc::new(Environment::new(2));

    let mut server = ServerBuilder::new(env)
        .register_service(CoordinatorService::create())
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

    Ok(())
}
