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

const METHOD_NUM_PROTO_SERVER_SAY_HELLO_NUM_PROTO: ::grpcio::Method<super::hellonumproto::NumProtoRequest, super::hellonumproto::NumProtoReply> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/hellonumproto.NumProtoServer/SayHelloNumProto",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

#[derive(Clone)]
pub struct NumProtoServerClient {
    client: ::grpcio::Client,
}

impl NumProtoServerClient {
    pub fn new(channel: ::grpcio::Channel) -> Self {
        NumProtoServerClient {
            client: ::grpcio::Client::new(channel),
        }
    }

    pub fn say_hello_num_proto_opt(&self, req: &super::hellonumproto::NumProtoRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::hellonumproto::NumProtoReply> {
        self.client.unary_call(&METHOD_NUM_PROTO_SERVER_SAY_HELLO_NUM_PROTO, req, opt)
    }

    pub fn say_hello_num_proto(&self, req: &super::hellonumproto::NumProtoRequest) -> ::grpcio::Result<super::hellonumproto::NumProtoReply> {
        self.say_hello_num_proto_opt(req, ::grpcio::CallOption::default())
    }

    pub fn say_hello_num_proto_async_opt(&self, req: &super::hellonumproto::NumProtoRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::hellonumproto::NumProtoReply>> {
        self.client.unary_call_async(&METHOD_NUM_PROTO_SERVER_SAY_HELLO_NUM_PROTO, req, opt)
    }

    pub fn say_hello_num_proto_async(&self, req: &super::hellonumproto::NumProtoRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::hellonumproto::NumProtoReply>> {
        self.say_hello_num_proto_async_opt(req, ::grpcio::CallOption::default())
    }
    pub fn spawn<F>(&self, f: F) where F: ::futures::Future<Item = (), Error = ()> + Send + 'static {
        self.client.spawn(f)
    }
}

pub trait NumProtoServer {
    fn say_hello_num_proto(&mut self, ctx: ::grpcio::RpcContext, req: super::hellonumproto::NumProtoRequest, sink: ::grpcio::UnarySink<super::hellonumproto::NumProtoReply>);
}

pub fn create_num_proto_server<S: NumProtoServer + Send + Clone + 'static>(s: S) -> ::grpcio::Service {
    let mut builder = ::grpcio::ServiceBuilder::new();
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_NUM_PROTO_SERVER_SAY_HELLO_NUM_PROTO, move |ctx, req, resp| {
        instance.say_hello_num_proto(ctx, req, resp)
    });
    builder.build()
}
