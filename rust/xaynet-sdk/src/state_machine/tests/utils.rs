use xaynet_core::{
    common::{RoundParameters, RoundSeed},
    crypto::{ByteObject, EncryptKeyPair, EncryptKeySeed, SigningKeyPair, SigningKeySeed},
    mask::{self, MaskConfig},
};

use crate::{settings::MaxMessageSize, state_machine::SharedState};

#[macro_export]
macro_rules! unwrap_as {
    ($e:expr, $p:path) => {
        match $e {
            $p(s) => s,
            x => panic!("Not a {}: {:?}", stringify!($p), x),
        }
    };
}

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
    ($phase:expr, $transition_outcome:path, $state_machine:path) => {{
        let x = $crate::unwrap_as!(
            $crate::state_machine::Step::step($phase).await,
            $transition_outcome
        );
        $crate::unwrap_as!(x, $state_machine)
    }};
}

#[macro_export]
macro_rules! unwrap_progress_continue {
    ($expr:expr) => {
        $crate::unwrap_as!($expr, $crate::state_machine::Progress::Continue)
    };
    ($phase:expr, $method:tt) => {
        unwrap_progress_continue!($phase.$method())
    };
    ($phase:expr, $method:tt, async) => {
        unwrap_progress_continue!($phase.$method().await)
    };
}

#[macro_export]
macro_rules! save_and_restore {
    ($phase:expr, $state:tt) => {{
        let mut phase = $phase;
        let io_mock = std::mem::replace(&mut phase.io, Box::new(MockIO::new()));
        let serializable_state = Into::<$crate::state_machine::SerializableState>::into(phase);
        // TODO: actually serialize the state here
        let state = $crate::unwrap_as!(
            serializable_state,
            $crate::state_machine::SerializableState::$state
        );
        let mut phase = $crate::state_machine::IntoPhase::<$state>::into_phase(state, io_mock);
        phase.check_io_mock();
        phase
    }};
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
        mask_config: mask_config().into(),
    }
}

pub fn shared_state(task: SelectFor) -> SharedState {
    SharedState {
        keys: SigningKeyPair::derive_from_seed(&SigningKeySeed::zeroed()),
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
