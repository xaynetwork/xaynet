# XayNet: federated learning made private, performant, and ubiquitous

###### tags: Xayn, Federated Learning, Privacy

This is the main source code repository for [xain-fl](https://www.xain.io/).

Developers, please check [CONTRIBUTING.md](./CONTRIBUTING.md)

---

> ### Want a framework that supports federated learning on the edge, in desktop browsers, integrates well with mobile apps, is performant, and preserves privacy? Welcome to XayNet, written entirely in Rust!

## Making federated learning easy for developers
Frameworks for machine learning - including those expressly for federated learning - exist already. These frameworks typically require the use of specific machine learning technology - for example tensorflow - or facilitate federated learning of cross-silo use cases - for example in collaborative learning across a limited number of hospitals.
We want to give developers more freedom of choice and abilities in the creation of federated learning software. By doing this, we hope to also increase the pace and scope of adoption of federated learning in practice.
Concretely, we provide developers with:
- [**My AI tools**] the flexibility to use the machine-learning frameworks and tools of their choice,
- [**My app dev tools**] the ability to integrate federated learning into apps written in Dart, Flutter or other languages of choice,
- [**“Federated learning” everywhere**] the ability to run federated learning everywhere - be it desktop browsers, smartphones or micro-controllers,
- [**“Federated learning” inside**] a simple integration means of making an AI application ready for federated learning,
- [**Privacy by design**] a communication protocol for federated learning that scales, is secure, and preserves the privacy of participating devices.

## The case for writing this framework in Rust

Rust has definitely potential as a host language for machine learning itself. But, above, we already insisted on giving developers freedom of choice here. Hence, we selected Rust for other reasons.
Our framework for federated learning is not a framework for machine learning as such. Rather, it supports the *federation* of machine learning that takes place on possibly heterogenuous devices and where use cases involve *many* such devices.
The programming language in which this framework is written should therefore give us strong support for the following:
- [**Compiles and runs “everywhere”**] The language should *not* require its own runtime and code should compile on a wide range of devices.
- [**Memory and Concurreny Safety**] Code that compiles should be both memory safe and free of data races.
- [**Secure communication**] State of the art cryptography should be available in vetted implementations.
- [**Asynchronous communication**] Abstractions for asynchronous communication should exist that make federated learning scale.
- [**Fast and functional**] The language should offer functional abstractions but also compile code into fast executables.
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