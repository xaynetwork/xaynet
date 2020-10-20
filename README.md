[![crates.io badge](https://img.shields.io/crates/v/xaynet.svg)](https://crates.io/crates/xaynet) [![docs.rs badge](https://docs.rs/xaynet/badge.svg)](https://docs.rs/xaynet) [![Coverage Status](https://codecov.io/gh/xaynetwork/xaynet/branch/master/graph/badge.svg)](https://codecov.io/gh/xaynetwork/xaynet)

![Xaynet banner](./assets/xaynet_banner.png)

# xaynet

## Xaynet: Train on the Edge with Federated Learning

Want a framework that supports federated learning on the edge, in
desktop browsers, integrates well with mobile apps, is performant, and
preserves privacy? Welcome to XayNet, written entirely in Rust!

### Making federated learning easy for developers

Frameworks for machine learning - including those expressly for
federated learning - exist already. These frameworks typically
facilitate federated learning of cross-silo use cases - for example in
collaborative learning across a limited number of hospitals or for
instance across multiple banks working on a common use case without
the need to share valuable and sensitive data.

This repository focusses on masked cross-device federated learning to
enable the orchestration of machine learning in millions of low-power
edge devices, such as smartphones or even cars. By doing this, we hope
to also increase the pace and scope of adoption of federated learning
in practice and especially allow the protection of end user data. All
data remains in private local premises, whereby only encrypted AI
models get automatically and asynchronously aggregated. Thus, we
provide a solution to the AI privacy dilemma and bridge the
often-existing gap between privacy and convenience. Imagine, for
example, a voice assistant to learn new words directly on device level
and sharing this knowledge with all other instances, without recording
and collecting your voice input centrally. Or, think about search
engine that learns to personalise search results without collecting
your often sensitive search queries centrally… There are thousands of
such use cases that right today still trade privacy for
convenience. We think this shouldn’t be the case and we want to
provide an alternative to overcome this dilemma.

Concretely, we provide developers with:

- **App dev tools**: An SDK to integrate federated learning into
  apps written in Dart or other languages of choice for mobile development,
  as well as frameworks like Flutter.
- **Privacy via cross-device federated learning**: Train your AI
  models locally on edge devices such as mobile phones, browsers,
  or even in cars. Federated learning automatically aggregates the
  local models into a global model. Thus, all insights inherent in
  the local models are captured, while the user data stays
  private on end devices.
- **Security Privacy via homomorphic encryption**: Aggregate
  models with the highest security and trust. Xayn’s masking
  protocol encrypts all models homomorphically. This enables you
  to aggregate encrypted local models into a global one – without
  having to decrypt local models at all. This protects private and
  even the most sensitive data.

### The case for writing this framework in Rust

Our framework for federated learning is not only a framework for
machine learning as such. Rather, it supports the federation of
machine learning that takes place on possibly heterogeneous devices
and where use cases involve many such devices.

The programming language in which this framework is written should
therefore give us strong support for the following:

- **Runs "everywhere"**: the language should not require its own
  runtime and code should compile on a wide range of devices.
- **Memory and concurrency safety**: code that compiles should be both
  memory safe and free of data races.
- **Secure communication**: state of the art cryptography should be
  available in vetted implementations.
- **Asynchronous communication**: abstractions for asynchronous
  communication should exist that make federated learning scale.
- **Fast and functional**: the language should offer functional
  abstractions but also compile code into fast executables.

Rust is one of the very few choices of modern programming languages
that meets these requirements:

- its concepts of Ownership and Borrowing make it both memory and
  thread-safe (hence avoiding many common concurrency issues).
- it has a strong and static type discipline and traits, which
  describe shareable functionality of a type.
- it is a modern systems programming language, with some functional
  style features such as pattern matching, closures and iterators.
- its idiomatic code compares favourably to idiomatic C in performance.
- it compiles to WASM and can therefore be applied natively in browser
  settings.
- it is widely deployable and doesn't necessarily depend on a runtime,
  unlike languages such as Java and their need for a virtual machine
  to run its code. Foreign Function Interfaces support calls from
  other languages/frameworks, including Dart, Python and Flutter.
- it compiles into LLVM, and so it can draw from the abundant tool
  suites for LLVM.

---

# Getting Started

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

**Please don't connect the coordinator to a Redis instance that is used in production!**

The coordinator clears the currently selected Redis database each time it is started.
This behavior will change as soon as the coordinator state can be automatically restored.

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
