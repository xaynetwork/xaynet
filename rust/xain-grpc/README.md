# XAIN gRPC communication in Rust

This crate holds Rust bindings generated from `.proto` files using a shell script:

```sh
./generate-proto-bindings.sh
```

Bindings are commited in this repository and we only need to re-run this script after modifying
`.proto` files.

To see the generated APIs, run:

```sh
cargo doc --open
```

## Coordinator

This is the main gRPC service and it uses TLS client authentication.

To test Coordinator, run the following two commands in separate command lines:

```sh
cargo run --bin coordinator_server -- -r certs/ca.cer -s certs/server.cer -k certs/server.key 
cargo run --bin coordinator_client -- -r certs/ca.cer -s certs/client.cer -k certs/client.key
```

## NumProto

This is just an example service for illustrative purposes and it doesn't use secure authentication.

To test NumProto, run the following two commands in separate command lines:

```sh
cargo run --bin numproto_server
cargo run --bin numproto_client
```
