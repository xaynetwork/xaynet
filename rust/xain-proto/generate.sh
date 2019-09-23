#!/bin/bash

set -e
cd "$(dirname "$0")"

if ! [[ -x "$(command -v grpc_rust_plugin)" ]]; then
    echo "Error: grpc_rust_plugin was not found"
    echo
    echo "To install, run: cargo install grpcio-compiler"
    exit 1
fi

proto_files="
../../numproto/numproto/protobuf/ndarray.proto
../../protobuf/xain/grpc/coordinator.proto
../../protobuf/xain/grpc/hellonumproto.proto
"

for proto in $proto_files; do
    echo "Processing: $proto"
    protoc \
        --rust_out=$PWD/src \
        --grpc_out=$PWD/src \
        --plugin=protoc-gen-grpc=`which grpc_rust_plugin` \
        --proto_path=../../numproto \
        --proto_path=../../protobuf/xain/grpc \
        $proto
done
