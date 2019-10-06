#!/bin/bash

set -e
cd "$(dirname "$0")"

if ! [[ -x "$(command -v grpc_rust_plugin)" ]]; then
    cargo install grpcio-compiler --version 0.4.3
fi

if ! [[ -x "$(command -v protobuf-bin-gen-rust-do-not-use)" ]]; then
    cargo install protobuf-codegen --version 2.8.1
fi

git submodule update --init --recursive

proto_files="
../../protobuf/xain/grpc/coordinator.proto
../../protobuf/xain/grpc/hellonumproto.proto
./numproto/numproto/protobuf/ndarray.proto
"

for proto in $proto_files; do
    echo "Processing: $proto"
    protoc \
        --rust_out=$PWD/src \
        --grpc_out=$PWD/src \
        --plugin=protoc-gen-grpc=`which grpc_rust_plugin` \
        --proto_path=./numproto \
        --proto_path=../../protobuf/xain/grpc \
        $proto
done
