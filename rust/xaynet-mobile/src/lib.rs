//! This crates provides a mobile friendly implementation of a Xaynet Federated Learning
//! participant, along with FFI C bindings for building applications in languages that
//! can use C bindings.
//!
//! The [`Participant`] provided by this crate is mobile friendly because the caller has
//! a lot of control on how to drive the participant execution. You can regularly pause
//! the execution of the participant, save it, and later restore it and continue the
//! execution. When running on a device that is low on battery or does not have access
//! to Wi-Fi for instance, it can be useful to be able to pause the participant.
//!
//! This control comes at a complexity cost though. Usually, a participant is split two:
//! - a task that executes a state machine that implements the PET protocol and emit
//!   notifications.
//! - a task that react to these events, for instance by downloading the latest global
//!   model at the end of a round, or trains a new model when the participant has been
//!   selected for the update task.
//!
//! The task that executes the PET protocol usually runs in background and we have
//! little control over it. This is a problem on mobile environment:
//! - first, the app may be killed at any moment and we'd lose the participant state
//! - second we don't really want a background task to potentially perform CPU heavy or
//!   network heavy operations without having a say since it may drain the battery or
//!   consume too much data.
//!
//! To solve this problem, the [`Participant`] provided in this crate embeds the PET
//! state machine, and it's the caller responsability to drive its execution (see
//! [`Participant::tick()`])
#[macro_use]
extern crate ffi_support;
#[macro_use]
extern crate async_trait;

#[macro_use]
extern crate tracing;

mod participant;
mod settings;
pub use self::{
    participant::{Event, Events, InitError, Notifier, Participant, Task},
    settings::{Settings, SettingsError},
};
pub mod ffi;

mod reqwest_client;
pub(crate) use reqwest_client::new_client;
pub use reqwest_client::ClientError;
