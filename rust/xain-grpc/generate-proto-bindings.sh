#!/bin/bash

set -e
cd "$(dirname "$0")"

if ! [[ -x "$(command -v grpc_rust_plugin)" ]]; then
    cargo install grpcio-compiler --version 0.4.3
fi

if ! [[ -x "$(command -v protobuf-bin-gen-rust-do-not-use)" ]]; then
    cargo install protobuf-codegen --version 2.8.1
fi

NUMPROTO_DIR=`python -m pip show numproto | grep Location | sed -e 's/^Location: //'`

PROTO_FILES="
../../protobuf/xain/grpc/coordinator.proto
../../protobuf/xain/grpc/hellonumproto.proto
$NUMPROTO_DIR/numproto/protobuf/ndarray.proto
"

for proto in $PROTO_FILES; do
    echo "Processing: $proto"
    protoc \
        --rust_out=$PWD/src/proto \
        --grpc_out=$PWD/src/proto \
        --plugin=protoc-gen-grpc=`which grpc_rust_plugin` \
        --proto_path=../../protobuf/xain/grpc \
        --proto_path=$NUMPROTO_DIR \
        $proto
done
