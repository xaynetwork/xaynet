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
    idle::{Idle, IdleStateError},
    phase::{Phase, PhaseName, PhaseState, Shared},
    shutdown::Shutdown,
    sum::{Sum, SumStateError},
    sum2::Sum2,
    unmask::{Unmask, UnmaskStateError},
    update::{Update, UpdateStateError},
};
