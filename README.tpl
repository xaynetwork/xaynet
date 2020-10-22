[![crates.io badge](https://img.shields.io/crates/v/xaynet.svg)](https://crates.io/crates/xaynet) [![docs.rs badge](https://docs.rs/xaynet/badge.svg)](https://docs.rs/xaynet)  [![rustc badge](https://img.shields.io/badge/rustc-1.46+-lightgray.svg)](https://www.rust-lang.org/learn/get-started) {{badges}}

![Xaynet banner](./assets/xaynet_banner.png)

# {{crate}}

{{readme}}

---

# Getting Started

## Minimum supported rust version

rustc 1.46.0

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

### Using `docker-compose`

The convenience of using the docker setup is that there's no need to setup a working Rust
environment on your system, as everything is done inside the container.

Start the coordinator by pointing to the `docker/docker-compose.yml` file. Keep in mind that
given this is the file used for development, it spins up some infrastructure that is currently
not essential.

```bash
docker-compose -f docker/docker-compose.yml up --build
```

If you would like, you can use the `docker/docker-compose-release.yml` file, but keep in mind
that given this runs a release build with optimizations, compilation will be slower.

```bash
docker-compose -f docker/docker-compose-release.yml up --build
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

The coordinator can be built and started with:

```bash
cargo run --bin coordinator --manifest-path rust/Cargo.toml -- -c configs/config.toml
```

## Running the example

The example can be found under [rust/examples/](./rust/examples/). It uses a dummy model
but is network-capable, so it's a good starting point for checking connectivity with
the coordinator.

### `test-drive-net.rs`

Make sure you have a running instance of the coordinator and that the clients
you will spawn with the command below are able to reach it through the network.

Here is an example on how to start `20` participants that will connect to a coordinator
running on `127.0.0.1:8081`:

```bash
cd rust
RUST_LOG=info cargo run --example test-drive-net -- -n 20 -u http://127.0.0.1:8081
```

For more in-depth details on how to run examples, see the accompanying Getting
Started guide under [rust/xaynet-server/src/examples.rs](./rust/xaynet-server/src/examples.rs).

## Troubleshooting

If you have any difficulties running the project, please reach out to us by
[opening an issue](https://github.com/xaynetwork/xaynet/issues/new) and describing your setup
and the problems you're facing.
