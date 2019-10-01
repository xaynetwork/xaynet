use log::error;

use xain_coordinator::training::{FromParticipant, InMessage};
use xain_proto::coordinator::{RendezvousReply, RendezvousRequest};
use xain_proto::coordinator_grpc::{self, Coordinator};

use futures::future::Future;
use futures_channel::mpsc;
use futures_util::stream::StreamExt;

type Sender<T> = mpsc::UnboundedSender<T>;
type Receiver<T> = mpsc::UnboundedReceiver<T>;

#[derive(Clone)]
pub struct CoordinatorService {
    sender: Sender<InMessage>,
}

impl CoordinatorService {
    pub fn create(sender: Sender<InMessage>) -> ::grpcio::Service {
        coordinator_grpc::create_coordinator(CoordinatorService { sender })
    }
}

impl Coordinator for CoordinatorService {
    fn rendezvous(
        &mut self,
        ctx: ::grpcio::RpcContext,
        req: RendezvousRequest,
        sink: ::grpcio::UnarySink<RendezvousReply>,
    ) {
        println!("Incoming request: {:?}", req);
        self.sender.unbounded_send(
            InMessage::Joined(FromParticipant { from: 1, payload: () })
        ).unwrap();

        let reply = RendezvousReply::default();
        let f = sink.success(reply).map_err(move |e| error!("failed to reply {:?}: {:?}", req, e));
        ctx.spawn(f)
    }
}
