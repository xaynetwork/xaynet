# https://github.com/rust-lang/docker-rust-nightly/blob/master/buster/Dockerfile
FROM buildpack-deps:stable-curl AS builder

# Install Rust
ENV RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:$PATH
RUN apt update
COPY docker/install_rust.sh .
RUN bash ./install_rust.sh

COPY rust/ .
RUN cargo build --bin coordinator --all-features

FROM ubuntu:20.04
RUN apt update && apt install -y --no-install-recommends libssl-dev
COPY --from=builder /target/debug/coordinator /app/coordinator
