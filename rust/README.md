# XAIN coordinator in Rust

The implemention of the XAIN coordinator in Rust.

## Authentication

The coordinator currently uses TLS client authentication for authenticating clients.

## Running the Server and the test client

The server project contains a set of certificates for local testing.

```sh
$ cd xain-grpc-api
$ cargo run --bin server -- -r certs/ca.cer -s certs/server.cer -k certs/server.key 
$ cargo run --bin client -- -r certs/ca.cer -s certs/client.cer -k certs/client.key
```
