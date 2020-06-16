use thiserror::Error;

use crate::message::DecodeError;

pub type ServiceError = anyhow::Error;

#[derive(Error, Debug)]
pub enum RequestFailed {
    #[error("Failed to decrypt the message with the coordinator secret key")]
    Decrypt,

    #[error("Parsing failed: {0:?}")]
    Parsing(DecodeError),

    #[error("Invalid message signature")]
    InvalidMessageSignature,

    #[error("Invalid sum signature")]
    InvalidSumSignature,

    #[error("Invalid update signature")]
    InvalidUpdateSignature,

    #[error("Not eligible for sum task")]
    NotSumEligible,

    #[error("Not eligible for update task")]
    NotUpdateEligible,

    #[error("The message was rejected because the coordinator did not expect it")]
    UnexpectedMessage,

    // TODO: we should have a retry layer that automatically retries
    // requests that fail with this error.
    #[error("The request could not be processed due to a temporary internal error")]
    TemporaryInternalError,

    #[error("TODO")]
    Other,
}
