pub mod coordinator;
pub mod message;
pub mod participant;
pub mod utils;

#[derive(Debug, PartialEq)]
pub enum PetError {
    InsufficientSystemEntropy,
    InvalidMessage,
}
