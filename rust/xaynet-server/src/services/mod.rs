//! This module implements the services the PET protocol provides.
//!
//! There are two main types of services:
//!
//! - the services for fetching data broadcasted by the state machine:
//!   - [`MaskLengthService`]: for fetching the length of the model
//!   - [`ModelService`]: for fetching the last available global model
//!   - [`RoundParamsService`]: for fetching the current round parameters
//!   - [`SeedDictService`]: for fetching the seed dictionary
//!   - [`SumDictService`]: for fetching the sum dictionary
//! - the services for handling PET messages from the participant:
//!   - [`MessageParserService`]: decrypt and parses incoming message
//!   - [`TaskValidator`]: performs sanity checks on the messages
//!     (verify the task signatures, etc.)
//!   - [`StateMachineService`]: pass the messages down to the state machine
//!     for actual processing
//!
//! The [`Fetcher`] trait provides a unified interface for the first
//! category of services. A [`Fetcher`] is a service that provides all
//! the subservices listed above. The [`PetMessageHandler`] trait is
//! an interface for the second category of services.
pub mod fetchers;
pub mod messages;

#[cfg(test)]
mod tests;
