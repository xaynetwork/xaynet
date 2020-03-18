#![feature(or_patterns)]
#![feature(bool_to_option)]

#[macro_use]
extern crate tracing;

#[macro_use]
extern crate serde;

pub mod aggregator;
pub mod common;
pub mod coordinator;
pub mod pet;
