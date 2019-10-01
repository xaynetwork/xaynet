use log::error;

use xain_proto::hellonumproto::{NumProtoRequest, NumProtoReply};
use xain_proto::hellonumproto_grpc::{self, NumProtoServer};

use futures::future::Future;

#[derive(Clone, Copy)]
pub struct NumProtoService;

impl NumProtoService {
    pub fn create() -> ::grpcio::Service {
        hellonumproto_grpc::create_num_proto_server(NumProtoService)
    }
}

impl NumProtoServer for NumProtoService {
    fn say_hello_num_proto(&mut self, ctx: ::grpcio::RpcContext, req: NumProtoRequest, sink: ::grpcio::UnarySink<NumProtoReply>) {
        println!("Incoming request: {:?}", req);
        let reply = NumProtoReply::default();
        let f = sink
            .success(reply)
            .map_err(move |e| error!("failed to reply {:?}: {:?}", req, e));
        ctx.spawn(f)
    }
}