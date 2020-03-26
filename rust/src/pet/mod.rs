pub mod coordinator;
pub mod participant;
pub mod utils;

#[derive(Debug, PartialEq)]
pub enum PetError {
    InvalidMessage,
}
