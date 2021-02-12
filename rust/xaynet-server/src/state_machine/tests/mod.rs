//! State machine test utilities.

pub mod coordinator_state;
pub mod event_bus;
pub mod impls;
pub mod initializer;
pub mod utils;

pub use coordinator_state::CoordinatorStateBuilder;
pub use event_bus::EventBusBuilder;

const WARNING: &str = "All state machine tests were written assuming these initial values.
First, carefully check the correctness of the state machine test before finally
changing these values.";
