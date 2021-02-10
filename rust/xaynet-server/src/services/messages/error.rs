use displaydoc::Display;
use thiserror::Error;

use crate::state_machine::requests::RequestError;
use xaynet_core::message::DecodeError;

/// Errors for the message parsing service.
#[derive(Debug, Display, Error)]
pub enum ServiceError {
    /// Failed to decrypt the message with the coordinator secret key.
    Decrypt,
    /// Failed to parse the message: {0}.
    Parsing(DecodeError),
    /// Invalid message signature.
    InvalidMessageSignature,
    /// Invalid coordinator public key.
    InvalidCoordinatorPublicKey,
    /// The message was not expected in the current phase.
    UnexpectedMessage,
    // FIXME: we need to refine the state machine errors and the
    // conversion into a service error
    /// The state machine failed to process the request: {0}.
    StateMachine(RequestError),
    /// Participant is not eligible for sum task.
    NotSumEligible,
    /// Participant is not eligible for update task.
    NotUpdateEligible,
    /// Internal error: {0}.
    InternalError(String),
}

impl From<Box<dyn std::error::Error>> for ServiceError {
    fn from(e: Box<dyn std::error::Error>) -> Self {
        match e.downcast::<ServiceError>() {
            Ok(e) => *e,
            Err(e) => ServiceError::InternalError(format!("{}", e)),
        }
    }
}

impl From<Box<dyn std::error::Error + Sync + Send>> for ServiceError {
    fn from(e: Box<dyn std::error::Error + Sync + Send>) -> Self {
        ServiceError::from(e as Box<dyn std::error::Error>)
    }
}
