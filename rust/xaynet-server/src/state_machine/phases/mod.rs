//! This module provides the states (aka phases) of the [`StateMachine`].
//!
//! [`StateMachine`]: crate::state_machine::StateMachine

mod failure;
mod handler;
mod idle;
mod phase;
mod shutdown;
mod sum;
mod sum2;
mod unmask;
mod update;

pub use self::{
    failure::{Failure, PhaseError},
    handler::Handler,
    idle::{Idle, IdleError},
    phase::{Phase, PhaseName, PhaseState, Shared},
    shutdown::Shutdown,
    sum::{Sum, SumError},
    sum2::Sum2,
    unmask::{Unmask, UnmaskError},
    update::{Update, UpdateError},
};
