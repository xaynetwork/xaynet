mod client;
mod heartbeat;
mod protocol;
mod service;

#[cfg(test)]
pub(crate) use self::service::ServiceRequests;
pub use self::service::{RequestError, Selector, Service, ServiceHandle};
