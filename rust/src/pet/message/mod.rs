#![allow(dead_code)] // temporary

pub mod round;
pub mod sum;
pub mod sum2;
pub mod update;

use std::ops::Range;

use sodiumoxide::crypto::box_;

use self::{round::RoundBox, sum::SumBox, sum2::Sum2Box, update::UpdateBox};
use crate::pet::PetError;

const ROUND_TAG: u8 = 100;
const SUM_TAG: u8 = 101;
const UPDATE_TAG: u8 = 102;
const SUM2_TAG: u8 = 103;
const TAG_RANGE: Range<usize> = 0..1;

const ROUNDBOX_RANGE: Range<usize> = 0..117;
const NONCE_RANGE: Range<usize> = 117..141;
const MESSAGEBOX_START: usize = 141;

enum MessageBox {
    Sum(SumBox),
    Update(UpdateBox, usize),
    Sum2(Sum2Box),
}

struct Message {
    roundbox: RoundBox,
    messagebox: MessageBox,
}

impl Message {
    fn seal(
        &self,
        coord_encr_pk: &box_::PublicKey,
        part_encr_sk: &box_::SecretKey,
    ) -> Result<Vec<u8>, PetError> {
        let roundbox = self.roundbox.seal(coord_encr_pk)?;
        let messagebox = match self.messagebox {
            MessageBox::Sum(ref sumbox) => sumbox.seal(coord_encr_pk, part_encr_sk)?,
            MessageBox::Update(ref updatebox, dict_sum_length) => {
                updatebox.seal(coord_encr_pk, part_encr_sk, dict_sum_length)?
            }
            MessageBox::Sum2(ref sum2box) => sum2box.seal(coord_encr_pk, part_encr_sk)?,
        };
        Ok([roundbox, messagebox].concat())
    }
}
