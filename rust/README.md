# XAIN coordinator in Rust

The implemention of the XAIN coordinator in Rust.

Coordinator currently uses TLS client authentication for authenticating clients.

To test Coordinator, run the following two commands in separate command lines:

```sh
cargo run --bin coordinator_server -- -r xain-grpc/certs/ca.cer -s xain-grpc/certs/server.cer -k xain-grpc/certs/server.key 
cargo run --bin coordinator_client -- -r xain-grpc/certs/ca.cer -s xain-grpc/certs/client.cer -k xain-grpc/certs/client.key
```

For more information, see the readme in [xain-grpc](xain-grpc).
