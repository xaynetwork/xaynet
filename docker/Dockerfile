# https://github.com/rust-lang/docker-rust-nightly/blob/master/buster/Dockerfile
FROM buildpack-deps:stable-curl AS builder

# Install Rust nightly
ENV RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:$PATH
RUN apt update
COPY docker/install_rust.sh .
RUN bash ./install_rust.sh

# Install dev tools
RUN apt install -y valgrind

# First, fetch and build all the dependencies by compiling an empty
# crate that depends on everything
COPY rust/Cargo.lock /
COPY rust/Cargo.toml /
RUN cargo fetch
RUN mkdir -p /src && \
    touch /src/lib.rs && \
    cargo build --lib --all-features

# Now copy the actual source code
COPY rust/src /src
# Fix timestamp. cargo incremental build as issues caused by the above
# trick to compile the deps separately
RUN touch /src/lib.rs
RUN cargo build --all-features

FROM alpine:latest
COPY --from=builder /target/debug/coordinator /app/coordinator
