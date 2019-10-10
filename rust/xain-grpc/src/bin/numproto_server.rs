use std::sync::Arc;
use std::thread;
use std::time::Duration;

use futures::future::Future;

use xain_grpc::proto::hellonumproto::{NumProtoReply, NumProtoRequest};
use xain_grpc::proto::hellonumproto_grpc::{create_num_proto_server, NumProtoServer};
use xain_grpc::proto::ndarray::NDArray;

type AppError = Box<dyn std::error::Error + Send + Sync>;

#[derive(Clone)]
pub struct NumProtoService;

impl NumProtoServer for NumProtoService {
    fn say_hello_num_proto(
        &mut self,
        ctx: grpcio::RpcContext,
        req: NumProtoRequest,
        sink: grpcio::UnarySink<NumProtoReply>,
    ) {
        // Get the `arr` field from the request.
        let arr: &[u8] = req.get_arr().get_ndarray();
        println!("NumProto server received: {:?}", arr);

        // Multiply `arr` by 2.
        let arr: Vec<u8> = arr.iter().map(|x| 2 * x).collect();
        println!("NumProto server sent: {:?}", arr);

        // Create a reply with the new value of `arr`.
        let mut reply = NumProtoReply::default();
        let mut nda = NDArray::new();
        nda.set_ndarray(arr);
        reply.set_arr(nda);

        // Create a future that sends the reply.
        let fut = sink.success(reply);
        // Print an error if sending the reply fails.
        let fut = fut.map_err(move |err| eprintln!("reply failed: {}", err));
        // Spawn the reply future.
        ctx.spawn(fut)
    }
}

fn main() -> Result<(), AppError> {
    // Create a gRPC event loop.
    let env = Arc::new(grpcio::Environment::new(2));

    // Start a NumProto server.
    let mut server = grpcio::ServerBuilder::new(env)
        .register_service(create_num_proto_server(NumProtoService))
        .bind("127.0.0.1", 50_051)
        .build()?;
    server.start();

    // Sleep forever.
    loop {
        thread::sleep(Duration::from_secs(1_000_000));
    }
}
