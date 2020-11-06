//! This module implements the services the PET protocol provides.
//!
//! There are two main types of services:
//!
//! - the services for fetching data broadcasted by the state
//!   machine. These services are implemented in the [`fetchers`]
//!   module
//! - the services for processing PET message are provided by the
//!   [`messages`] module.

pub mod fetchers;
pub mod messages;

#[cfg(test)]
mod tests;
