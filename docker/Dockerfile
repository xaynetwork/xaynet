FROM buildpack-deps:stable-curl AS builder

RUN apt update

# Install Rust
ENV RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:$PATH
RUN curl https://sh.rustup.rs -sSf | sh -s -- -y --profile minimal

# install build dependencies: libc, openssl
RUN apt install -y build-essential libssl-dev pkg-config

COPY rust/ /rust/
WORKDIR /rust/xaynet-server

# https://github.com/linkerd/linkerd2-proxy/blob/main/Dockerfile#L31

# Controls which profile the coordinator is compiled with.
# If set to RELEASE_BUILD=1, the coordinator is compiled using the release profile.
# Default is development profile.
ARG RELEASE_BUILD=0

# Controls which optional features the coordinator is compiled with.
# Syntax:
# default features:     -
# single feature:       COORDINATOR_FEATURES=tls
# multiple features:    COORDINATOR_FEATURES=tls,metrics
# all features:         COORDINATOR_FEATURES=full
ARG COORDINATOR_FEATURES

RUN mkdir -p /out && \
  echo "RELEASE_BUILD=$RELEASE_BUILD COORDINATOR_FEATURES=$COORDINATOR_FEATURES" && \
  if [ "$RELEASE_BUILD" -eq "0" ]; \
  then \
    cargo build --features="$COORDINATOR_FEATURES" && \
    mv /rust/target/debug/coordinator /out/coordinator; \
  else \
    cargo build --features="$COORDINATOR_FEATURES" --release && \
    mv /rust/target/release/coordinator /out/coordinator; \
  fi

FROM ubuntu:20.04
RUN apt update && apt install -y --no-install-recommends libssl-dev
COPY --from=builder /out/coordinator /app/coordinator

ENTRYPOINT ["/app/coordinator", "-c", "/app/config.toml"]
