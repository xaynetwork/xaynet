#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/xaynetwork/xaynet/master/assets/logo.png",
    issue_tracker_base_url = "https://github.com/tokio-rs/tracing/issues/"
)]
//! # Xaynet: Train on the Edge with Federated Learning
//!
//! ###### tags: Xayn, Federated Learning, Privacy, edge AI, Machine Learning, mobile AI
//!
//! Want a framework that supports federated learning on the edge, in
//! desktop browsers, integrates well with mobile apps, is performant, and
//! preserves privacy? Welcome to XayNet, written entirely in Rust!
//!
//! ## Making federated learning easy for developers
//!
//! Frameworks for machine learning - including those expressly for
//! federated learning - exist already. These frameworks typically
//! facilitate federated learning of cross-silo use cases - for example in
//! collaborative learning across a limited number of hospitals or for
//! instance across multiple banks working on a common use case without
//! the need to share valuable and sensitive data.
//!
//! This repository focusses on masked cross-device federated learning to
//! enable the orchestration of machine learning in millions of low-power
//! edge devices, such as smartphones or even cars. By doing this, we hope
//! to also increase the pace and scope of adoption of federated learning
//! in practice and especially allow the protection of end user data. All
//! data remains in private local premises, whereby only encrypted AI
//! models get automatically and asynchronously aggregated. Thus, we
//! provide a solution to the AI privacy dilemma and bridge the
//! often-existing gap between privacy and convenience. Imagine, for
//! example, a voice assistant to learn new words directly on device level
//! and sharing this knowledge with all other instances, without recording
//! and collecting your voice input centrally. Or, think about search
//! engine that learns to personalise search results without collecting
//! your often sensitive search queries centrally… There are thousands of
//! such use cases that right today still trade privacy for
//! convenience. We think this shouldn’t be the case and we want to
//! provide an alternative to overcome this dilemma.
//!
//! Concretely, we provide developers with:
//!
//! - **App dev tools**: An SDK to integrate federated learning into
//!   apps written in Dart or other languages of choice for mobile development,
//!   as well as frameworks like Flutter.
//! - **Privacy via cross-device federated learning**: Train your AI
//!   models locally on edge devices such as mobile phones, browsers,
//!   or even in cars. Federated learning automatically aggregates the
//!   local models into a global model. Thus, all insights inherent in
//!   the local models are captured, while the user data stays
//!   private on end devices.
//! - **Security Privacy via homomorphic encryption**: Aggregate
//!   models with the highest security and trust. Xayn’s masking
//!   protocol encrypts all models homomorphically. This enables you
//!   to aggregate encrypted local models into a global one – without
//!   having to decrypt local models at all. This protects private and
//!   even the most sensitive data.
//!
//! ## The case for writing this framework in Rust
//!
//! Our framework for federated learning is not only a framework for
//! machine learning as such. Rather, it supports the federation of
//! machine learning that takes place on possibly heterogeneous devices
//! and where use cases involve many such devices.
//!
//! The programming language in which this framework is written should
//! therefore give us strong support for the following:
//!
//! - **Runs "everywhere"**: the language should not require its own
//!   runtime and code should compile on a wide range of devices.
//! - **Memory and concurrency safety**: code that compiles should be both
//!   memory safe and free of data races.
//! - **Secure communication**: state of the art cryptography should be
//!   available in vetted implementations.
//! - **Asynchronous communication**: abstractions for asynchronous
//!   communication should exist that make federated learning scale.
//! - **Fast and functional**: the language should offer functional
//!   abstractions but also compile code into fast executables.
//!
//! Rust is one of the very few choices of modern programming languages
//! that meets these requirements:
//!
//! - its concepts of Ownership and Borrowing make it both memory and
//!   thread-safe (hence avoiding many common concurrency issues).
//! - it has a strong and static type discipline and traits, which
//!   describe shareable functionality of a type.
//! - it is a modern systems programming language, with some functional
//!   style features such as pattern matching, closures and iterators.
//! - its idiomatic code compares favourably to idiomatic C in performance.
//! - it compiles to WASM and can therefore be applied natively in browser
//!   settings.
//! - it is widely deployable and doesn't necessarily depend on a runtime,
//!   unlike languages such as Java and their need for a virtual machine
//!   to run its code. Foreign Function Interfaces support calls from
//!   other languages/frameworks, including Dart, Python and Flutter.
//! - it compiles into LLVM, and so it can draw from the abundant tool
//!   suites for LLVM.
#[macro_use]
extern crate serde;

pub mod common;
pub mod crypto;
pub mod mask;
pub mod message;

use std::collections::HashMap;

use thiserror::Error;

use self::crypto::{
    encrypt::{PublicEncryptKey, SecretEncryptKey},
    sign::{PublicSigningKey, SecretSigningKey, Signature},
};

#[derive(Error, Debug)]
#[error("initialization failed: insufficient system entropy to generate secrets")]
/// An error related to insufficient system entropy for secrets at program startup.
pub struct InitError;

/// A public encryption key that identifies a coordinator.
pub type CoordinatorPublicKey = PublicEncryptKey;

/// A secret encryption key that belongs to the public key of a
/// coordinator.
pub type CoordinatorSecretKey = SecretEncryptKey;

/// A public signature key that identifies a participant.
pub type ParticipantPublicKey = PublicSigningKey;

/// A secret signature key that belongs to the public key of a
/// participant.
pub type ParticipantSecretKey = SecretSigningKey;

/// A public signature key that identifies a sum participant.
pub type SumParticipantPublicKey = ParticipantPublicKey;

/// A secret signature key that belongs to the public key of a sum
/// participant.
pub type SumParticipantSecretKey = ParticipantSecretKey;

/// A public encryption key generated by a sum participant. It is used
/// by the update participants to encrypt their masking seed for each
/// sum participant.
pub type SumParticipantEphemeralPublicKey = PublicEncryptKey;

/// The secret counterpart of [`SumParticipantEphemeralPublicKey`]
pub type SumParticipantEphemeralSecretKey = SecretEncryptKey;

/// A public signature key that identifies an update participant.
pub type UpdateParticipantPublicKey = ParticipantPublicKey;

/// A secret signature key that belongs to the public key of an update
/// participant.
pub type UpdateParticipantSecretKey = ParticipantSecretKey;

/// A signature to prove a participant's eligibility for a task.
pub type ParticipantTaskSignature = Signature;

/// A dictionary created during the sum phase of the protocol. It maps the public key of every sum
/// participant to the ephemeral public key generated by that sum participant.
pub type SumDict = HashMap<SumParticipantPublicKey, SumParticipantEphemeralPublicKey>;

/// Local seed dictionaries are sent by update participants. They contain the participant's masking
/// seed, encrypted with the ephemeral public key of each sum participant.
pub type LocalSeedDict = HashMap<SumParticipantPublicKey, mask::seed::EncryptedMaskSeed>;

/// A dictionary created during the update phase of the protocol. The global seed dictionary is
/// built from the local seed dictionaries sent by the update participants. It maps each sum
/// participant to the encrypted masking seeds of all the update participants.
pub type SeedDict = HashMap<SumParticipantPublicKey, UpdateSeedDict>;

/// Values of [`SeedDict`]. Sent to sum participants.
pub type UpdateSeedDict = HashMap<UpdateParticipantPublicKey, mask::seed::EncryptedMaskSeed>;
