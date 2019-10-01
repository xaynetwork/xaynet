use log::error;

use xain_proto::coordinator::{RendezvousReply, RendezvousRequest};
use xain_proto::coordinator_grpc::{self, Coordinator};

use futures::future::Future;

#[derive(Clone, Copy)]
pub struct CoordinatorService;

impl CoordinatorService {
    pub fn create() -> ::grpcio::Service {
        coordinator_grpc::create_coordinator(CoordinatorService)
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
        let reply = RendezvousReply::default();
        let f = sink.success(reply).map_err(move |e| error!("failed to reply {:?}: {:?}", req, e));
        ctx.spawn(f)
    }
}
