use std::fs;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use async_std::task;
use clap::{App, Arg};
use futures::future::Future;
use futures_channel::mpsc;
use grpcio::{Environment, ServerBuilder, ServerCredentialsBuilder};

use xain_coordinator::training::{FromParticipant, InMessage};
use xain_grpc::proto::coordinator::{
    HeartbeatReply, HeartbeatRequest, RendezvousReply, RendezvousRequest,
};
use xain_grpc::proto::coordinator_grpc::{create_coordinator, Coordinator};
use xain_grpc::training_task::TrainingTask;

type DynError = Box<dyn std::error::Error + Send + Sync>;

type Sender<T> = mpsc::UnboundedSender<T>;
#[allow(dead_code)]
type Receiver<T> = mpsc::UnboundedReceiver<T>;

#[derive(Clone)]
pub struct CoordinatorService {
    sender: Sender<InMessage>,
}

impl Coordinator for CoordinatorService {
    fn rendezvous(
        &mut self,
        ctx: grpcio::RpcContext,
        _req: RendezvousRequest,
        sink: grpcio::UnarySink<RendezvousReply>,
    ) {
        log::info!("Rendezvous");

        self.sender
            .unbounded_send(InMessage::Joined(FromParticipant { from: 1, payload: () }))
            .unwrap();

        // Spawn a future that sends a reply.
        let fut = sink.success(RendezvousReply::default());
        let fut = fut.map_err(|err| log::error!("reply failed: {:?}", err));
        ctx.spawn(fut)
    }

    fn heartbeat(
        &mut self,
        ctx: grpcio::RpcContext,
        _req: HeartbeatRequest,
        sink: grpcio::UnarySink<HeartbeatReply>,
    ) {
        log::info!("Heartbeat");

        // Spawn a future that sends a reply.
        let fut = sink.success(HeartbeatReply::default());
        let fut = fut.map_err(|err| log::error!("reply failed: {:?}", err));
        ctx.spawn(fut)
    }
}

fn main() -> Result<(), DynError> {
    // Set up logging.
    env_logger::Builder::new().filter_level(log::LevelFilter::Info).init();
    grpcio::redirect_log();

    // Parse arguments passed to the program.
    let args = App::new("xain-coordinator")
        .version("0.1")
        .about("The coordinator for the XAIN distributed ML framework!")
        .author("The XAIN developers")
        .arg(Arg::with_name("root-cert").short("r").required(true).takes_value(true))
        .arg(Arg::with_name("server-cert").short("s").required(true).takes_value(true))
        .arg(Arg::with_name("server-key").short("k").required(true).takes_value(true))
        .get_matches();

    // Load certificates.
    let root_cert = fs::read_to_string(args.value_of("root-cert").unwrap())?;
    let server_cert = fs::read_to_string(args.value_of("server-cert").unwrap())?;
    let private_key = fs::read_to_string(args.value_of("server-key").unwrap())?;
    let credentials = ServerCredentialsBuilder::new()
        .root_cert(root_cert.into_bytes(), true)
        .add_cert(server_cert.into_bytes(), private_key.into_bytes())
        .build();

    // Create gRPC event loop.
    let env = Arc::new(Environment::new(2));

    // Start the training task.
    let (mut training_task, sender) = TrainingTask::create();
    task::spawn(async move {
        training_task.run().await;
    });

    // Start Coordinator server.
    let mut server = ServerBuilder::new(env)
        .register_service(create_coordinator(CoordinatorService { sender }))
        .bind_secure("127.0.0.1", 50_051, credentials)
        .build()?;
    server.start();

    // Sleep forever.
    loop {
        thread::sleep(Duration::from_secs(1_000_000));
    }
}
