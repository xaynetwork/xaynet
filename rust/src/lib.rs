#![allow(dead_code)]
#![allow(unused_imports)]
#![feature(or_patterns)]
#![feature(bool_to_option)]

#[macro_use]
extern crate tracing;

#[macro_use]
extern crate serde;

pub mod coordinator;
pub mod message;
pub mod participant;
pub mod service;
pub mod utils;

#[derive(Debug, PartialEq)]
pub enum PetError {
    InsufficientSystemEntropy,
    InvalidMessage,
    InsufficientParticipants,
}
