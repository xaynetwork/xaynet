#!/bin/bash

set -e

cd controller
cargo test

rustup component add rustfmt
cargo fmt -- --check
