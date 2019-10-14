// This file is generated. Do not edit
// @generated

// https://github.com/Manishearth/rust-clippy/issues/702
#![allow(unknown_lints)]
#![allow(clippy::all)]

#![cfg_attr(rustfmt, rustfmt_skip)]

#![allow(box_pointers)]
#![allow(dead_code)]
#![allow(missing_docs)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(trivial_casts)]
#![allow(unsafe_code)]
#![allow(unused_imports)]
#![allow(unused_results)]

const METHOD_COORDINATOR_RENDEZVOUS: ::grpcio::Method<super::coordinator::RendezvousRequest, super::coordinator::RendezvousReply> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/xain.protobuf.coordinator.Coordinator/Rendezvous",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_COORDINATOR_HEARTBEAT: ::grpcio::Method<super::coordinator::HeartbeatRequest, super::coordinator::HeartbeatReply> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/xain.protobuf.coordinator.Coordinator/Heartbeat",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

#[derive(Clone)]
pub struct CoordinatorClient {
    client: ::grpcio::Client,
}

impl CoordinatorClient {
    pub fn new(channel: ::grpcio::Channel) -> Self {
        CoordinatorClient {
            client: ::grpcio::Client::new(channel),
        }
    }

    pub fn rendezvous_opt(&self, req: &super::coordinator::RendezvousRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::coordinator::RendezvousReply> {
        self.client.unary_call(&METHOD_COORDINATOR_RENDEZVOUS, req, opt)
    }

    pub fn rendezvous(&self, req: &super::coordinator::RendezvousRequest) -> ::grpcio::Result<super::coordinator::RendezvousReply> {
        self.rendezvous_opt(req, ::grpcio::CallOption::default())
    }

    pub fn rendezvous_async_opt(&self, req: &super::coordinator::RendezvousRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::coordinator::RendezvousReply>> {
        self.client.unary_call_async(&METHOD_COORDINATOR_RENDEZVOUS, req, opt)
    }

    pub fn rendezvous_async(&self, req: &super::coordinator::RendezvousRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::coordinator::RendezvousReply>> {
        self.rendezvous_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn heartbeat_opt(&self, req: &super::coordinator::HeartbeatRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::coordinator::HeartbeatReply> {
        self.client.unary_call(&METHOD_COORDINATOR_HEARTBEAT, req, opt)
    }

    pub fn heartbeat(&self, req: &super::coordinator::HeartbeatRequest) -> ::grpcio::Result<super::coordinator::HeartbeatReply> {
        self.heartbeat_opt(req, ::grpcio::CallOption::default())
    }

    pub fn heartbeat_async_opt(&self, req: &super::coordinator::HeartbeatRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::coordinator::HeartbeatReply>> {
        self.client.unary_call_async(&METHOD_COORDINATOR_HEARTBEAT, req, opt)
    }

    pub fn heartbeat_async(&self, req: &super::coordinator::HeartbeatRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::coordinator::HeartbeatReply>> {
        self.heartbeat_async_opt(req, ::grpcio::CallOption::default())
    }
    pub fn spawn<F>(&self, f: F) where F: ::futures::Future<Item = (), Error = ()> + Send + 'static {
        self.client.spawn(f)
    }
}

pub trait Coordinator {
    fn rendezvous(&mut self, ctx: ::grpcio::RpcContext, req: super::coordinator::RendezvousRequest, sink: ::grpcio::UnarySink<super::coordinator::RendezvousReply>);
    fn heartbeat(&mut self, ctx: ::grpcio::RpcContext, req: super::coordinator::HeartbeatRequest, sink: ::grpcio::UnarySink<super::coordinator::HeartbeatReply>);
}

pub fn create_coordinator<S: Coordinator + Send + Clone + 'static>(s: S) -> ::grpcio::Service {
    let mut builder = ::grpcio::ServiceBuilder::new();
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_COORDINATOR_RENDEZVOUS, move |ctx, req, resp| {
        instance.rendezvous(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_COORDINATOR_HEARTBEAT, move |ctx, req, resp| {
        instance.heartbeat(ctx, req, resp)
    });
    builder.build()
}
