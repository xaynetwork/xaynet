# XayNet: federated learning made private, performant, and ubiquitous

###### tags: Xayn, Federated Learning, Privacy

This is the main source code repository for [xain-fl](https://www.xain.io/).

![crates.io badge](https://img.shields.io/crates/v/xain-fl.svg) ![docs.rs badge](https://docs.rs/xain-fl/badge.svg?version=0.8.0) ![crates.io downloads](https://img.shields.io/crates/d/xain-fl.svg)

---

> ### Want a framework that supports federated learning on the edge, in desktop browsers, integrates well with mobile apps, is performant, and preserves privacy? Welcome to XayNet, written entirely in Rust!

## Making federated learning easy for developers
Frameworks for machine learning - including those expressly for federated learning - exist already. These frameworks typically require the use of specific machine learning technology - for example tensorflow - or facilitate federated learning of cross-silo use cases - for example in collaborative learning across a limited number of hospitals.

We want to give developers more freedom of choice and abilities in the creation of federated learning software. By doing this, we hope to also increase the pace and scope of adoption of federated learning in practice.

Concretely, we provide developers with:
- **My AI tools:** The flexibility to use the machine-learning frameworks and tools of their choice.
- **My app dev tools:** The ability to integrate federated learning into apps written in Dart, Flutter or other languages of choice.
- **"Federated learning" everywhere:**: The ability to run federated learning everywhere - be it desktop browsers, smartphones or micro-controllers.
- **"Federated learning" inside:**: A simple integration means of making an AI application ready for federated learning.
- **Privacy by design:** A communication protocol for federated learning that scales, is secure, and preserves the privacy of participating devices.

## The case for writing this framework in Rust

Rust has definitely potential as a host language for machine learning itself. But, above, we already insisted on giving developers freedom of choice here. Hence, we selected Rust for other reasons.

Our framework for federated learning is not a framework for machine learning as such. Rather, it supports the *federation* of machine learning that takes place on possibly heterogenuous devices and where use cases involve *many* such devices.

The programming language in which this framework is written should therefore give us strong support for the following:
- **Compiles and runs "everywhere":** The language should *not* require its own runtime and code should compile on a wide range of devices.
- **Memory and Concurreny Safety:** Code that compiles should be both memory safe and free of data races.
- **Secure communication:** State of the art cryptography should be available in vetted implementations.
- **Asynchronous communication:** Abstractions for asynchronous communication should exist that make federated learning scale.
- **Fast and functional:** The language should offer functional abstractions but also compile code into fast executables.

Rust is one of the very few choices of modern programming languages that meet these requirements:
- Its concepts of *Ownership* and *Borrowing* make it both memory and concurreny safe.
- It has a strong and static type discipline and traits, which describe shareable functionality of a type.
- It has rich functional abstractions, for example the `tower-service` based on the foundational trait `Service`.
- Its Idiomatic code compares favorably to Idiomatic C in performance.
- It has no run-time and so is widely deployable. Foreign Function Interfaces support calls from other languages, including Dart or Flutter.
- And it compiles into LLVM, and so it can draw from the abundant tool suites for LLMV.

## We love XayNet, we like to hear about your use of it

We feel blessed to have such a strong Engineering team that includes several senior Rust developers and folks who were eager to become experienced Rust programmers themselves! All of us are excited to share the fruits of this labor with you.

So without further ado, here is the release of XayNet, our federated learning framework written entirely in Rust. We hope you will like and use this framework. And we will be grateful for any feedback, contributions or news on your usage of XayNet in your own projects.

---

# Getting Started

## Running the platform

There are a few different ways to run the backend: via docker, or by deploying it to
a Kubernetes cluster or by compiling the code and running the binary manually.

1. Everything described below assumes your shell's working directory to be the root
of the repository.
2. The following instructions assume you have pre-existing knowledge on some
of the referenced software (like `docker` and `docker-compose`) and/or a working
setup (if you decide on compiling the Rust code and running the binary manually)
3. In case you need help with setting up your system accordingly, we recommend you
refer to the official documentation of each tool, as supporting them here would be
beyond the scope of this project:
   * [Rust](https://www.rust-lang.org/tools/install)
   * [Docker](https://docs.docker.com/) and [Docker Compose](https://docs.docker.com/compose/)
   * [Kubernetes](https://kubernetes.io/docs/home/)

### Using `docker-compose`

The conveniency of using the docker setup is that there's no need to setup a working Rust
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
through the manifests and adjust them according to your own setup (namespace, ingress etc).

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

The example can be found under [rust/src/bin/](./rust/src/bin/). It uses a dummy model
and operates via network, so it's a good starting point for checking connectivity with
the coordinator.

### `test-drive-net.rs`

Make sure you have a running instance of the coordinator and that the clients that
you will spawn with the command below are able to reach it through the network.

Here is an example on how to start `20` participants that will connect to a coordinator
running on `127.0.0.1:8081`:

```bash
RUST_LOG=xain_fl=info cargo run --bin test-drive-net -- -n 20 -u http://127.0.0.1:8081
```

## Troubleshooting

If you have any dificulties running the project, please reach out to us by
[opening an issue](https://github.com/xainag/xain-fl/issues/new) and describing your setup
and the problems you're facing.
