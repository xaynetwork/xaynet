[![crates.io badge](https://img.shields.io/crates/v/xaynet.svg)](https://crates.io/crates/xaynet) [![docs.rs badge](https://docs.rs/xaynet/badge.svg)](https://docs.rs/xaynet) [![rustc badge](https://img.shields.io/badge/rustc-1.48+-lightgray.svg)](https://www.rust-lang.org/learn/get-started) {{badges}} [![roadmap badge](https://img.shields.io/badge/Roadmap-2021-blue)](./ROADMAP.md)

![Xaynet banner](./assets/xaynet_banner.png)

# {{crate}}

{{readme}}

---

# Getting Started

## Minimum supported rust version

rustc 1.48.0

## Running the platform

There are a few different ways to run the backend: via docker, or by deploying it to
a Kubernetes cluster or by compiling the code and running the binary manually.

1. Everything described below assumes your shell's working directory to be the root
of the repository.
2. The following instructions assume you have pre-existing knowledge on some
of the referenced software (like `docker` and `docker-compose`) and/or a working
setup (if you decide to compile the Rust code and run the binary manually).
3. In case you need help with setting up your system accordingly, we recommend you
refer to the official documentation of each tool, as supporting them here would be
beyond the scope of this project:
   * [Rust](https://www.rust-lang.org/tools/install)
   * [Docker](https://docs.docker.com/) and [Docker Compose](https://docs.docker.com/compose/)
   * [Kubernetes](https://kubernetes.io/docs/home/)

**Note:**

With Xaynet `v0.11` the coordinator needs a connection to a redis instance in order to save its state.

**Don't connect the coordinator to a Redis instance that is used in production!**

We recommend connecting the coordinator to its own Redis instance. We have invested a lot of
time to make sure that the coordinator only deletes its own data but in the current state of
development, we cannot guarantee that this will always be the case.

### Using Docker

The convenience of using the docker setup is that there's no need to setup a working Rust
environment on your system, as everything is done inside the container.

#### Run an image from Docker Hub

Docker images of the latest releases are provided on
[Docker Hub](https://hub.docker.com/r/xaynetwork/xaynet).

You can try them out with the default `configs/docker-dev.toml` by running:

**Xaynet below v0.11**

```bash
docker run -v ${PWD}/configs/docker-dev.toml:/app/config.toml -p 8081:8081 xaynetwork/xaynet:v0.10.0 /app/coordinator -c /app/config.toml
```

**Xaynet v0.11+**

```bash
# don't forget to adjust the Redis url in configs/docker-dev.toml
docker run -v ${PWD}/configs/docker-dev.toml:/app/config.toml -p 8081:8081 xaynetwork/xaynet:v0.11.0
```

The docker image contains a release build of the coordinator without optional features.

#### Run a coordinator with additional infrastructure

Start the coordinator by pointing to the `docker/docker-compose.yml` file. It spins up all
infrastructure that is essential to run the coordinator with default or optional features.
Keep in mind that this file is used for development only.

```bash
docker-compose -f docker/docker-compose.yml up --build
```

#### Create a release build

If you would like, you can create an optimized release build of the coordinator,
but keep in mind that the compilation will be slower.

```bash
docker build --build-arg RELEASE_BUILD=1 -f ./docker/Dockerfile .
```

#### Build a coordinator with optional features

Optional features can be specified via the build argument `COORDINATOR_FEATURES`.

```bash
docker build --build-arg COORDINATOR_FEATURES=tls,metrics -f ./docker/Dockerfile .
```

### Using Kubernetes

To deploy an instance of the coordinator to your Kubernetes cluster, use the manifests that are
located inside the `k8s/coordinator` folder. The manifests rely on `kustomize` to be generated
(`kustomize` is officially supported by `kubectl` since v1.14). We recommend you thoroughly go
through the manifests and adjust them according to your own setup (namespace, ingress, etc.).

Remember to also check (and adjust if necessary) the default configuration for the coordinator, available
at `k8s/coordinator/development/config.toml`.

Please adjust the domain used in the `k8s/coordinator/development/ingress.yaml` file so it matches
your needs (you can also skip `ingress` altogether, just make sure you remove its reference from
`k8s/coordinator/development/kustomization.yaml`).

Keep in mind that the `ingress` configuration that is shown on `k8s/coordinator/development/ingress.yaml`
relies on resources that aren't available in this repository, due to their sensitive nature
(TLS key and certificate, for instance).

To verify the generated manifests, run:

```bash
kubectl kustomize k8s/coordinator/development
```

To apply them:

```bash
kubectl apply -k k8s/coordinator/development
```

In case you are not exposing your coordinator via `ingress`, you can still reach it using a port-forward.
The example below creates a port-forward at port `8081` assuming the coordinator pod is still using the
`app=coordinator` label:

```bash
kubectl port-forward $(kubectl get pods -l "app=coordinator" -o jsonpath="{.items[0].metadata.name}") 8081
```

### Building the project manually

The coordinator without optional features can be built and started with:

```bash
cd rust
cargo run --bin coordinator -- -c ../configs/config.toml
```

## Running the example

The example can be found under [rust/examples/](./rust/examples/). It uses a dummy model
but is network-capable, so it's a good starting point for checking connectivity with
the coordinator.

### `test-drive`

Make sure you have a running instance of the coordinator and that the clients
you will spawn with the command below are able to reach it through the network.

Here is an example on how to start `20` participants that will connect to a coordinator
running on `127.0.0.1:8081`:

```bash
cd rust
RUST_LOG=info cargo run --example test-drive -- -n 20 -u http://127.0.0.1:8081
```

For more in-depth details on how to run examples, see the accompanying Getting
Started guide under [rust/xaynet-server/src/examples.rs](./rust/xaynet-server/src/examples.rs).

## Troubleshooting

If you have any difficulties running the project, please reach out to us by
[opening an issue](https://github.com/xaynetwork/xaynet/issues/new) and describing your setup
and the problems you're facing.
