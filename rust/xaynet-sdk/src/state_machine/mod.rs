// Important the macro_use modules must be declared first for the
// macro to be used in the other modules (until declarative macros are stable)
#[macro_use]
mod phase;
mod io;
mod phases;
#[allow(clippy::module_inception)]
mod state_machine;

// It is useful to re-export everything within this module because
// there are lot of interdependencies between all the sub-modules
#[cfg(test)]
use self::io::MockIO;
use self::{
    io::{boxed_io, IO},
    phase::{IntoPhase, Phase, PhaseIo, Progress, SharedState, State, Step},
    phases::{Awaiting, NewRound, Sum, Sum2, Update},
};

pub use self::{
    phase::SerializableState,
    state_machine::{StateMachine, TransitionOutcome},
};

#[cfg(test)]
pub(crate) mod testutils {
    use xaynet_core::{
        common::{RoundParameters, RoundSeed},
        crypto::{ByteObject, EncryptKeyPair, EncryptKeySeed, SigningKeyPair, SigningKeySeed},
        mask::{self, MaskConfig},
    };

    use crate::settings::MaxMessageSize;

    use super::*;

    #[macro_export]
    macro_rules! unwrap_step {
        ($phase:expr, complete, $state_machine:tt) => {
            unwrap_step!(
                $phase,
                $crate::state_machine::TransitionOutcome::Complete,
                $state_machine
            )
        };
        ($phase:expr, pending, $state_machine:tt) => {
            unwrap_step!(
                $phase,
                $crate::state_machine::TransitionOutcome::Pending,
                $state_machine
            )
        };
        ($phase:expr, $transition_outcome:path, awaiting) => {
            unwrap_step!(
                $phase,
                $transition_outcome,
                $crate::state_machine::StateMachine::Awaiting
            )
        };
        ($phase:expr, $transition_outcome:path, sum) => {
            unwrap_step!(
                $phase,
                $transition_outcome,
                $crate::state_machine::StateMachine::Sum
            )
        };
        ($phase:expr, $transition_outcome:path, sum2) => {
            unwrap_step!(
                $phase,
                $transition_outcome,
                $crate::state_machine::StateMachine::Sum2
            )
        };
        ($phase:expr, $transition_outcome:path, update) => {
            unwrap_step!(
                $phase,
                $transition_outcome,
                $crate::state_machine::StateMachine::Update
            )
        };
        ($phase:expr, $transition_outcome:path, $state_machine:path) => {
            match Step::step($phase).await {
                $transition_outcome(state_machine) => match state_machine {
                    $state_machine(phase) => phase,
                    x => panic!("unexpected state machine phase: {:?}", x),
                },
                x => panic!("unexpected transition outcome {:?}", x),
            }
        };
    }

    #[macro_export]
    macro_rules! unwrap_progress_continue {
        ($phase:expr, $method:tt) => {
            match $phase.$method().await {
                Progress::Continue(phase) => phase,
                x => panic!("expected Progress::Continue, got {:?}", x),
            }
        };
    }

    /// Task for which the round parameters should be generated.
    #[derive(Debug, PartialEq, Eq)]
    pub enum SelectFor {
        /// Create round parameters that always select participants for the sum task
        Sum,
        /// Create round parameters that always select participants for the update task
        Update,
        /// Create round parameters that never select participants
        None,
    }

    pub fn mask_config() -> MaskConfig {
        MaskConfig {
            group_type: mask::GroupType::Prime,
            data_type: mask::DataType::F32,
            bound_type: mask::BoundType::B0,
            model_type: mask::ModelType::M3,
        }
    }

    pub fn round_params(task: SelectFor) -> RoundParameters {
        RoundParameters {
            pk: EncryptKeySeed::zeroed().derive_encrypt_key_pair().0,
            sum: if task == SelectFor::Sum { 1.0 } else { 0.0 },
            update: if task == SelectFor::Update { 1.0 } else { 0.0 },
            seed: RoundSeed::zeroed(),
        }
    }

    pub fn shared_state(task: SelectFor) -> SharedState {
        SharedState {
            keys: SigningKeyPair::derive_from_seed(&SigningKeySeed::zeroed()),
            mask_config: mask_config().into(),
            scalar: 1.0,
            message_size: MaxMessageSize::unlimited(),
            round_params: round_params(task),
        }
    }

    pub struct EncryptKeyGenerator(EncryptKeySeed);

    impl EncryptKeyGenerator {
        pub fn new() -> Self {
            Self(EncryptKeySeed::zeroed())
        }

        fn incr_seed(&mut self) {
            let mut raw = self.0.as_slice().to_vec();
            for b in &mut raw {
                if *b < 0xff {
                    *b += 1;
                    return self.0 = EncryptKeySeed::from_slice(raw.as_slice()).unwrap();
                }
            }
            panic!("max seed");
        }

        pub fn next(&mut self) -> EncryptKeyPair {
            let keys = EncryptKeyPair::derive_from_seed(&self.0);
            self.incr_seed();
            keys
        }
    }

    pub struct SigningKeyGenerator(SigningKeySeed);

    impl SigningKeyGenerator {
        pub fn new() -> Self {
            Self(SigningKeySeed::zeroed())
        }

        fn incr_seed(&mut self) {
            let mut raw = self.0.as_slice().to_vec();
            for b in &mut raw {
                if *b < 0xff {
                    *b += 1;
                    return self.0 = SigningKeySeed::from_slice(raw.as_slice()).unwrap();
                }
            }
            panic!("max seed");
        }

        pub fn next(&mut self) -> SigningKeyPair {
            let item = SigningKeyPair::derive_from_seed(&self.0);
            self.incr_seed();
            item
        }
    }
}
