#!/bin/bash

set -e

export RUSTFLAGS="-D warnings"
export CARGO_INCREMENTAL=0

cd controller
cargo test

rustup component add rustfmt
cargo fmt -- --check
