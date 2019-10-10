use std::sync::Arc;

use xain_grpc::proto::hellonumproto::NumProtoRequest;
use xain_grpc::proto::hellonumproto_grpc::NumProtoServerClient;
use xain_grpc::proto::ndarray::NDArray;

type AppError = Box<dyn std::error::Error + Send + Sync>;

fn main() -> Result<(), AppError> {
    // Create a gRPC event loop.
    let env = Arc::new(grpcio::Environment::new(2));

    // Start a NumProto client.
    let channel = grpcio::ChannelBuilder::new(env).connect("localhost:50051");
    let client = NumProtoServerClient::new(channel);

    // Create a request.
    let mut req = NumProtoRequest::new();
    let mut nda = NDArray::new();
    let arr = vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
    nda.set_ndarray(arr.clone());
    req.set_arr(nda);
    println!("NumProto client sent: {:?}", arr);

    // Send the request and wait for the reply.
    let reply = client.say_hello_num_proto(&req)?;
    let arr: &[u8] = reply.get_arr().get_ndarray();
    println!("NumProto client received: {:?}", arr);

    Ok(())
}
