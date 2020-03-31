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
const CERTIFICATE_RANGE: Range<usize> = 1..1;
const SIGN_SUM_RANGE: Range<usize> = 1..65;

const ROUNDBOX_RANGE: Range<usize> = 0..117;
const NONCE_RANGE: Range<usize> = 117..141;
const MESSAGEBOX_START: usize = 141;

trait BufferRef<'a> {
    fn bytes(&self) -> &'a [u8];

    fn tag(&self) -> &'a [u8] {
        &self.bytes()[TAG_RANGE]
    }

    fn certificate(&self) -> &'a [u8] {
        &self.bytes()[CERTIFICATE_RANGE]
    }

    fn signature_sum(&self) -> &'a [u8] {
        &self.bytes()[SIGN_SUM_RANGE]
    }
}

trait BufferMut {
    fn bytes_mut(&mut self) -> &mut [u8];

    fn tag_mut(&mut self) -> &mut [u8] {
        &mut self.bytes_mut()[TAG_RANGE]
    }

    fn certificate_mut(&mut self) -> &mut [u8] {
        &mut self.bytes_mut()[CERTIFICATE_RANGE]
    }

    fn signature_sum_mut(&mut self) -> &mut [u8] {
        &mut self.bytes_mut()[SIGN_SUM_RANGE]
    }
}

enum MsgBox {
    Sum(SumBox),
    Update(UpdateBox),
    Sum2(Sum2Box),
}

struct Message {
    roundbox: RoundBox,
    messagebox: MsgBox,
}

impl Message {
    fn seal(&self, coord_encr_pk: &box_::PublicKey, part_encr_sk: &box_::SecretKey) -> Vec<u8> {
        let roundbox = self.roundbox.seal(coord_encr_pk);
        let messagebox = match self.messagebox {
            MsgBox::Sum(ref sumbox) => sumbox.seal(coord_encr_pk, part_encr_sk),
            MsgBox::Update(ref updatebox) => updatebox.seal(coord_encr_pk, part_encr_sk),
            MsgBox::Sum2(ref sum2box) => sum2box.seal(coord_encr_pk, part_encr_sk),
        };
        [roundbox, messagebox].concat()
    }
}
