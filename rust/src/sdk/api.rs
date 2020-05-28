use std::{
    cmp::Ordering,
    iter::FromIterator,
    os::raw::{c_double, c_float, c_int, c_long},
    ptr::null,
};

use num::{bigint::BigInt, rational::Ratio, traits::Zero};

use crate::{
    mask::model::{FromPrimitives, IntoPrimitives, Model},
    participant::{Participant, Task},
};

// TODO: this is a mock, replaced by sth like the `Client` from #397
pub struct Client {
    participant: Participant,

    // counting starts from 1, 0 means not seen yet
    current_round: usize,
    checked_round: usize,

    // pointer to primitive models are valid for the current round
    // TODO: set this to None when a new round is observed
    primitive_model_f32: Option<Vec<f32>>,
    primitive_model_f64: Option<Vec<f64>>,
    primitive_model_i32: Option<Vec<i32>>,
    primitive_model_i64: Option<Vec<i64>>,
}

macro_rules! get_model {
    ($suffix:ident, $rust:ty, $c:ty $(,)?) => {
        paste::item! {
            #[no_mangle]
            /// Get a pointer to the latest global model, which is valid until the next round
            /// starts. Returns a `null` pointer if no global model is available or type casting
            /// fails.
            pub extern "C" fn [<get_model $suffix>](&mut self) -> *const $c {
                // TODO: this is a mock, get the model from the round params when #411 is merged
                let model = Some(Model::from_iter(
                    vec![Ratio::<BigInt>::zero(); 10].into_iter(),
                ));

                if let Some(ref m) = self.[<primitive_model $suffix>] {
                    return m.as_ptr();
                }
                if let Some(m) = model {
                    self.[<primitive_model $suffix>] = m
                        .into_primitives()
                        .map(|res| res.map_err(|_| ()))
                        .collect::<Result<Vec<$rust>, ()>>()
                        .ok();
                    if let Some(ref m) = self.[<primitive_model $suffix>] {
                        return m.as_ptr();
                    }
                }
                null()
            }
        }
    };
}

impl Client {
    #[no_mangle]
    /// Check if the next round has started.
    pub extern "C" fn is_next_round(&mut self) -> bool {
        // TODO: increment the `current_round` if the client sees a new coordinator pk
        // as a result of `self.handle.get_round_parameters().await`
        match self.checked_round.cmp(&self.current_round) {
            // new round since the last check
            Ordering::Less => {
                self.checked_round = self.current_round;
                true
            }
            // still same round since the last check
            Ordering::Equal => false,
            // should only ever happen if the client is reused for another FL use case
            Ordering::Greater => panic!("restart the participant for a new FL use case"),
        }
    }

    #[no_mangle]
    /// Check if the current role of the participant is `update`.
    pub extern "C" fn is_update_participant(&self) -> bool {
        self.participant.task == Task::Update
    }

    get_model!(_f32, f32, c_float);
    get_model!(_f64, f64, c_double);
    get_model!(_i32, i32, c_int);
    get_model!(_i64, i64, c_long);
}
