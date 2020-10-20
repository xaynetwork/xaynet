use super::{Participant, ParticipantState};
use crate::mobile_client::participant::{sum::Sum, update::Update, Role};
use xaynet_core::crypto::Signature;

type SumSignature = Signature;
type UpdateSignature = Signature;
#[derive(Serialize, Deserialize, Clone)]
pub struct Awaiting;

impl Participant<Awaiting> {
    pub fn new(state: ParticipantState) -> Self {
        Self {
            inner: Awaiting,
            state,
        }
    }

    /// Check eligibility for a task given probabilities for `Sum` and `Update`
    /// selection in this round.
    ///
    /// Returns the participant [`Role`] selected for this round.
    pub fn determine_role(self, round_seed: &[u8], round_sum: f64, round_update: f64) -> Role {
        let (sum_signature, update_signature) = self.compute_signatures(round_seed);
        if sum_signature.is_eligible(round_sum) {
            Participant::<Sum>::new(self.state, sum_signature).into()
        } else if update_signature.is_eligible(round_update) {
            Participant::<Update>::new(self.state, sum_signature, update_signature).into()
        } else {
            Participant::<Awaiting>::new(self.state).into()
        }
    }

    /// Compute the sum and update signatures for the given round seed.
    fn compute_signatures(&self, round_seed: &[u8]) -> (SumSignature, UpdateSignature) {
        (
            self.state
                .keys
                .secret
                .sign_detached(&[round_seed, b"sum"].concat()),
            self.state
                .keys
                .secret
                .sign_detached(&[round_seed, b"update"].concat()),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mobile_client::participant::{AggregationConfig, MaxMessageSize};
    use sodiumoxide::randombytes::randombytes;
    use xaynet_core::{
        crypto::{ByteObject, SigningKeyPair},
        mask::{BoundType, DataType, GroupType, MaskConfig, ModelType},
        ParticipantPublicKey,
        ParticipantSecretKey,
    };

    fn participant_state() -> ParticipantState {
        sodiumoxide::init().unwrap();

        let aggregation_config = AggregationConfig {
            mask: MaskConfig {
                group_type: GroupType::Prime,
                data_type: DataType::F32,
                bound_type: BoundType::B0,
                model_type: ModelType::M3,
            },

            scalar: 1_f64,
        };
        ParticipantState {
            keys: SigningKeyPair::generate(),
            aggregation_config,
            max_message_size: MaxMessageSize::default(),
        }
    }

    #[test]
    fn test_compute_signature() {
        let part = Participant::<Awaiting>::new(participant_state());
        let round_seed = randombytes(32);
        let (sum_signature, update_signature) = part.compute_signatures(&round_seed);
        assert!(part
            .state
            .keys
            .public
            .verify_detached(&sum_signature, &[round_seed.as_slice(), b"sum"].concat(),));
        assert!(part.state.keys.public.verify_detached(
            &update_signature,
            &[round_seed.as_slice(), b"update"].concat(),
        ));
    }

    #[test]
    fn test_determine_role_sum() {
        let mut state = participant_state();
        state.keys.public = ParticipantPublicKey::from_slice_unchecked(&[
            190, 240, 238, 150, 126, 52, 119, 102, 54, 206, 171, 29, 119, 90, 211, 80, 72, 227,
            201, 206, 171, 91, 251, 194, 247, 255, 169, 255, 38, 58, 178, 237,
        ]);
        state.keys.secret = ParticipantSecretKey::from_slice_unchecked(&[
            28, 220, 233, 161, 16, 15, 83, 189, 203, 121, 65, 252, 33, 102, 213, 151, 187, 211, 73,
            50, 152, 229, 253, 23, 113, 38, 135, 62, 75, 10, 105, 149, 190, 240, 238, 150, 126, 52,
            119, 102, 54, 206, 171, 29, 119, 90, 211, 80, 72, 227, 201, 206, 171, 91, 251, 194,
            247, 255, 169, 255, 38, 58, 178, 237,
        ]);

        let part = Participant::<Awaiting>::new(state);
        let eligible_sum_seed = &[
            119, 119, 241, 118, 43, 216, 159, 35, 122, 253, 138, 162, 8, 248, 64, 153, 163, 160,
            193, 111, 216, 217, 127, 168, 104, 99, 42, 55, 201, 207, 226, 237,
        ];

        match part.determine_role(eligible_sum_seed, 0.5_f64, 0.5_f64) {
            Role::Summer(_) => (),
            _ => panic!(),
        }
    }

    #[test]
    fn test_determine_role_sum_2() {
        let mut state = participant_state();
        state.keys.public = ParticipantPublicKey::from_slice_unchecked(&[
            122, 57, 133, 117, 137, 93, 73, 153, 3, 27, 117, 89, 92, 108, 163, 15, 38, 173, 212,
            172, 14, 197, 65, 43, 58, 136, 55, 214, 247, 25, 51, 141,
        ]);
        state.keys.secret = ParticipantSecretKey::from_slice_unchecked(&[
            165, 199, 74, 92, 27, 218, 120, 82, 31, 169, 158, 81, 40, 83, 5, 104, 238, 195, 129,
            111, 146, 245, 105, 137, 28, 86, 130, 219, 16, 192, 57, 209, 122, 57, 133, 117, 137,
            93, 73, 153, 3, 27, 117, 89, 92, 108, 163, 15, 38, 173, 212, 172, 14, 197, 65, 43, 58,
            136, 55, 214, 247, 25, 51, 141,
        ]);

        let part = Participant::<Awaiting>::new(state);
        let eligible_sum_update_seed = &[
            151, 199, 161, 82, 158, 218, 250, 94, 62, 82, 63, 10, 136, 239, 178, 177, 140, 128,
            170, 245, 38, 85, 161, 86, 143, 96, 18, 89, 161, 186, 172, 199,
        ];

        match part.determine_role(eligible_sum_update_seed, 0.5_f64, 0.5_f64) {
            Role::Summer(_) => (),
            _ => panic!(),
        }
    }

    #[test]
    fn test_determine_role_update() {
        let mut state = participant_state();
        state.keys.public = ParticipantPublicKey::from_slice_unchecked(&[
            201, 12, 132, 6, 110, 178, 107, 236, 29, 72, 101, 46, 204, 123, 52, 230, 234, 32, 170,
            15, 129, 0, 45, 37, 241, 184, 213, 12, 91, 31, 138, 194,
        ]);
        state.keys.secret = ParticipantSecretKey::from_slice_unchecked(&[
            161, 49, 83, 187, 114, 93, 66, 108, 38, 55, 116, 120, 141, 139, 63, 143, 111, 151, 222,
            191, 94, 194, 29, 222, 246, 42, 130, 139, 20, 6, 245, 192, 201, 12, 132, 6, 110, 178,
            107, 236, 29, 72, 101, 46, 204, 123, 52, 230, 234, 32, 170, 15, 129, 0, 45, 37, 241,
            184, 213, 12, 91, 31, 138, 194,
        ]);

        let part = Participant::<Awaiting>::new(state);
        let eligible_update_seed = &[
            138, 154, 233, 12, 24, 151, 168, 241, 106, 193, 49, 13, 179, 26, 193, 253, 32, 197, 62,
            80, 43, 96, 255, 29, 236, 183, 96, 245, 36, 182, 239, 179,
        ];

        match part.determine_role(eligible_update_seed, 0.5_f64, 0.5_f64) {
            Role::Updater(_) => (),
            _ => panic!(),
        }
    }

    #[test]
    fn test_determine_role_unselected() {
        let mut state = participant_state();
        state.keys.public = ParticipantPublicKey::from_slice_unchecked(&[
            236, 187, 56, 0, 180, 225, 181, 143, 195, 223, 136, 225, 92, 226, 111, 63, 146, 52,
            130, 249, 206, 31, 7, 112, 155, 138, 60, 179, 32, 138, 144, 129,
        ]);
        state.keys.secret = ParticipantSecretKey::from_slice_unchecked(&[
            29, 157, 49, 179, 55, 148, 27, 227, 251, 68, 22, 137, 145, 204, 123, 1, 49, 171, 163,
            134, 54, 76, 50, 79, 99, 166, 84, 99, 57, 94, 64, 117, 236, 187, 56, 0, 180, 225, 181,
            143, 195, 223, 136, 225, 92, 226, 111, 63, 146, 52, 130, 249, 206, 31, 7, 112, 155,
            138, 60, 179, 32, 138, 144, 129,
        ]);

        let part = Participant::<Awaiting>::new(state);
        let ineligible_sum_update_seed = &[
            95, 250, 161, 81, 73, 135, 223, 39, 247, 166, 154, 140, 93, 160, 137, 39, 248, 135,
            187, 119, 128, 151, 223, 57, 144, 229, 66, 150, 100, 75, 62, 62,
        ];

        match part.determine_role(ineligible_sum_update_seed, 0.5_f64, 0.5_f64) {
            Role::Unselected(_) => (),
            _ => panic!(),
        }
    }
}
