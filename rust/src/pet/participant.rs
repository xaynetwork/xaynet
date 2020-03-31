#![allow(dead_code)] // temporary

use std::{collections::HashMap, default::Default};

use sodiumoxide::{
    self,
    crypto::{box_, sealedbox, sign},
    randombytes::randombytes,
};

use super::{utils::is_eligible, PetError};

#[derive(Debug, PartialEq)]
/// Tasks of a participant.
enum Task {
    Sum,
    Update,
    None,
}

/// A participant in the PET protocol layer.
pub struct Participant {
    // credentials
    pub encr_pk: box_::PublicKey,      // 32 bytes
    encr_sk: box_::SecretKey,          // 32 bytes
    pub sign_pk: sign::PublicKey,      // 32 bytes
    sign_sk: sign::SecretKey,          // 64 bytes
    pub ephm_pk: box_::PublicKey,      // 32 bytes
    ephm_sk: box_::SecretKey,          // 32 bytes
    certificate: Vec<u8>,              // 0 bytes (dummy)
    signature_sum: sign::Signature,    // 64 bytes
    signature_update: sign::Signature, // 64 bytes

    // other
    task: Task,
}

impl Participant {
    pub fn new() -> Result<Self, PetError> {
        // crucial: init must be called before anything else in this module
        sodiumoxide::init()
            .and(Ok(Default::default()))
            .or(Err(PetError::InsufficientSystemEntropy))
    }

    /// Compute the "sum" and "update" signatures.
    pub fn compute_signatures(&mut self, round_seed: &[u8]) {
        self.signature_sum = sign::sign_detached(&[round_seed, b"sum"].concat(), &self.sign_sk);
        self.signature_update =
            sign::sign_detached(&[round_seed, b"update"].concat(), &self.sign_sk);
    }

    /// Check eligibility for a task.
    pub fn check_task(&mut self, round_sum: f64, round_update: f64) {
        if is_eligible(&self.signature_sum, round_sum) {
            self.task = Task::Sum;
        } else if is_eligible(&self.signature_update, round_update) {
            self.task = Task::Update;
        } else {
            self.task = Task::None;
        }
    }

    /// Compose a "sum" message.
    pub fn compose_sum_message(&mut self, coord_encr_pk: &box_::PublicKey) -> SumMessage {
        SumMessage::compose(self, coord_encr_pk)
    }

    /// Compose an "update" message.
    pub fn compose_update_message(
        &self,
        coord_encr_pk: &box_::PublicKey,
        dict_sum: &HashMap<box_::PublicKey, box_::PublicKey>,
    ) -> UpdateMessage {
        UpdateMessage::compose(self, coord_encr_pk, dict_sum)
    }

    /// Compose a "sum2" message.
    pub fn compose_sum2_message(
        &self,
        coord_encr_pk: &box_::PublicKey,
        dict_seed: &HashMap<box_::PublicKey, HashMap<box_::PublicKey, Vec<u8>>>,
    ) -> Result<Sum2Message, PetError> {
        Sum2Message::compose(self, coord_encr_pk, dict_seed)
    }
}

impl Default for Participant {
    fn default() -> Self {
        let (encr_pk, encr_sk) = box_::gen_keypair();
        let (sign_pk, sign_sk) = sign::gen_keypair();
        let ephm_pk = box_::PublicKey([0_u8; box_::PUBLICKEYBYTES]);
        let ephm_sk = box_::SecretKey([0_u8; box_::SECRETKEYBYTES]);
        let certificate: Vec<u8> = Vec::new();
        let signature_sum = sign::Signature([0_u8; sign::SIGNATUREBYTES]);
        let signature_update = sign::Signature([0_u8; sign::SIGNATUREBYTES]);
        let task = Task::None;
        Self {
            encr_pk,
            encr_sk,
            sign_pk,
            sign_sk,
            ephm_pk,
            ephm_sk,
            certificate,
            signature_sum,
            signature_update,
            task,
        }
    }
}

// Message egress with buffers:
//
//     encr_pk >-┐
//     sign_pk >-┤
//  SealedBoxBuffer >-┐
//             SealedBox >-┐
//      certificate >-┐    |
//    signature_sum >-┤    |
//          ephm_pk >-┤    |
//          SumBoxBuffer >-┤
//                     SumBox >-┐
//                   MessageBuffer >-┐
//                           SumMessage
//
//     encr_pk >-┐
//     sign_pk >-┤
//  SealedBoxBuffer >-┐
//             SealedBox >-┐
//      certificate >-┐    |
//    signature_sum >-┤    |
// signature_update >-┤    |
//        model_url >-┤    |
//        dict_seed >-┤    |
//       UpdateBoxBuffer >-┤
//                  UpdateBox >-┐
//                   MessageBuffer >-┐
//                        UpdateMessage
//
//     encr_pk >-┐
//     sign_pk >-┤
//  SealedBoxBuffer >-┐
//             SealedBox >-┐
//      certificate >-┐    |
//    signature_sum >-┤    |
//         mask_url >-┤    |
//         Sum2BoxBuffer >-┤
//                    Sum2Box >-┐
//                   MessageBuffer >-┐
//                          Sum2Message

/// Buffer and wrap the asymmetrically encrypted part of a "sum/update/sum2" message.
struct SealedBoxBuffer<'tag, 'encr_key, 'sign_key>(&'tag [u8], &'encr_key [u8], &'sign_key [u8]);

impl<'tag, 'encr_key, 'sign_key> SealedBoxBuffer<'tag, 'encr_key, 'sign_key> {
    fn new(encr_pk: &'encr_key box_::PublicKey, sign_pk: &'sign_key sign::PublicKey) -> Self {
        Self(
            b"round",         // 5 bytes
            encr_pk.as_ref(), // 32 bytes
            sign_pk.as_ref(), // 32 bytes
        ) // 69 bytes in total
    }

    fn seal(&self, coord_encr_pk: &box_::PublicKey) -> Vec<u8> {
        sealedbox::seal(&[self.0, self.1, self.2].concat(), coord_encr_pk) // 48 + 69 bytes, 117 bytes in total
    }
}

/// Buffer and wrap the symmetrically encrypted part of a "sum" message.
struct SumBoxBuffer<'tag, 'cert, 'sign_, 'ephm_key>(
    &'tag [u8],
    &'cert [u8],
    &'sign_ [u8],
    &'ephm_key [u8],
);

impl<'tag, 'cert, 'sign_, 'ephm_key> SumBoxBuffer<'tag, 'cert, 'sign_, 'ephm_key> {
    fn new(
        certificate: &'cert [u8],
        signature_sum: &'sign_ sign::Signature,
        ephm_pk: &'ephm_key box_::PublicKey,
    ) -> Self {
        Self(
            b"sum",                 // 3 bytes
            certificate,            // 0 bytes (dummy)
            signature_sum.as_ref(), // 64 bytes
            ephm_pk.as_ref(),       // 32 bytes
        ) // 99 bytes in total
    }

    fn seal(&self, coord_encr_pk: &box_::PublicKey, part_encr_sk: &box_::SecretKey) -> Vec<u8> {
        let nonce = box_::gen_nonce(); // 24 bytes
        let sumbox = box_::seal(
            &[self.0, self.1, self.2, self.3].concat(),
            &nonce,
            coord_encr_pk,
            part_encr_sk,
        ); // 16 + 99 bytes
        [nonce.as_ref(), &sumbox].concat() // 139 bytes in total
    }
}

/// Buffer and wrap the symmetrically encrypted part of an "update" message.
struct UpdateBoxBuffer<'tag, 'cert, 'sign_, 'url, 'dict>(
    &'tag [u8],
    &'cert [u8],
    &'sign_ [u8],
    &'sign_ [u8],
    &'url [u8],
    &'dict [u8],
);

impl<'tag, 'cert, 'sign_, 'url, 'dict> UpdateBoxBuffer<'tag, 'cert, 'sign_, 'url, 'dict> {
    fn new(
        certificate: &'cert [u8],
        signature_sum: &'sign_ sign::Signature,
        signature_update: &'sign_ sign::Signature,
        model_url: &'url [u8],
        dict_seed: &'dict [u8],
    ) -> Self {
        Self(
            b"update",                 // 6 bytes
            certificate,               // 0 bytes (dummy)
            signature_sum.as_ref(),    // 64 bytes
            signature_update.as_ref(), // 64 bytes
            model_url,                 // 32 bytes (dummy)
            dict_seed,                 // 112 * dict_sum.len() bytes
        ) // 166 + 112 * dict_sum.len() bytes in total
    }

    fn seal(&self, coord_encr_pk: &box_::PublicKey, part_encr_sk: &box_::SecretKey) -> Vec<u8> {
        let nonce = box_::gen_nonce(); // 24 bytes
        let updatebox = box_::seal(
            &[self.0, self.1, self.2, self.3, self.4, self.5].concat(),
            &nonce,
            coord_encr_pk,
            part_encr_sk,
        ); // 16 + 166 + 112 * dict_sum.len() bytes
        [nonce.as_ref(), &updatebox].concat() // 206 + 112 * dict_sum.len() bytes in total
    }
}

/// Buffer and wrap the symmetrically encrypted part of a "sum2" message.
struct Sum2BoxBuffer<'tag, 'cert, 'sign_, 'url>(&'tag [u8], &'cert [u8], &'sign_ [u8], &'url [u8]);

impl<'tag, 'cert, 'sign_, 'url> Sum2BoxBuffer<'tag, 'cert, 'sign_, 'url> {
    fn new(
        certificate: &'cert [u8],
        signature_sum: &'sign_ sign::Signature,
        mask_url: &'url [u8],
    ) -> Self {
        Self(
            b"sum2",                // 4 bytes
            certificate,            // 0 bytes (dummy)
            signature_sum.as_ref(), // 64 bytes
            mask_url,               // 32 bytes (dummy)
        ) // 100 bytes in total
    }

    fn seal(&self, coord_encr_pk: &box_::PublicKey, part_encr_sk: &box_::SecretKey) -> Vec<u8> {
        let nonce = box_::gen_nonce(); // 24 bytes
        let sum2box = box_::seal(
            &[self.0, self.1, self.2, self.3].concat(),
            &nonce,
            coord_encr_pk,
            part_encr_sk,
        ); // 16 + 100 bytes
        [nonce.as_ref(), &sum2box].concat() // 140 bytes in total
    }
}

/// Buffer and wrap an encrypted "sum/update/sum2" message.
struct MessageBuffer<'sbox, 'box___>(&'sbox [u8], &'box___ [u8]);

impl<'sbox, 'box___> MessageBuffer<'sbox, 'box___> {
    fn new(sealedbox: &'sbox [u8], box__: &'box___ [u8]) -> Self {
        Self(sealedbox, box__)
    }

    fn seal(&self) -> Vec<u8> {
        [self.0, self.1].concat()
    }
}

/// A "sum" message.
pub struct SumMessage {
    pub message: Vec<u8>, // 256 bytes
}

impl SumMessage {
    /// Generate an ephemeral asymmetric key pair and encrypt the "sum" message parts. Eligibility
    /// for the "sum" task should be checked beforehand.
    fn compose(part: &mut Participant, coord_encr_pk: &box_::PublicKey) -> Self {
        // generate ephemeral key pair
        let (ephm_pk, ephm_sk) = box_::gen_keypair();
        part.ephm_pk = ephm_pk;
        part.ephm_sk = ephm_sk;

        // encrypt message parts
        let sbox = SealedBoxBuffer::new(&part.encr_pk, &part.sign_pk).seal(coord_encr_pk);
        let sumbox = SumBoxBuffer::new(&part.certificate, &part.signature_sum, &part.ephm_pk)
            .seal(coord_encr_pk, &part.encr_sk);
        let message = MessageBuffer::new(&sbox, &sumbox).seal();

        Self { message }
    }
}

/// An "update" message and a seed for a mask of a local model.
pub struct UpdateMessage {
    pub message: Vec<u8>,   // 323 + 112 * dict_sum.len() bytes
    pub mask_seed: Vec<u8>, // 32 bytes
}

impl UpdateMessage {
    /// Mask a trained local model, create a dictionary of encrypted masking seeds and encrypt the
    /// "update" message parts. Eligibility for the "update" task should be checked beforehand.
    fn compose(
        part: &Participant,
        coord_encr_pk: &box_::PublicKey,
        dict_sum: &HashMap<box_::PublicKey, box_::PublicKey>,
    ) -> Self {
        // mask the local model
        let mask_seed = randombytes(32);
        let model_url = randombytes(32); // dummy

        // create dictionary of encrypted masking seeds
        let mut dict_seed: Vec<u8> = Vec::new();
        for (sum_encr_pk, sum_ephm_pk) in dict_sum.iter() {
            dict_seed.extend_from_slice(sum_encr_pk.as_ref()); // 32 bytes
            dict_seed.extend(sealedbox::seal(&mask_seed, sum_ephm_pk)); // 48 + 32 bytes
        } // 112 * dict_sum.len() bytes in total

        // encrypt message parts
        let sbox = SealedBoxBuffer::new(&part.encr_pk, &part.sign_pk).seal(coord_encr_pk);
        let updatebox = UpdateBoxBuffer::new(
            &part.certificate,
            &part.signature_sum,
            &part.signature_update,
            &model_url,
            &dict_seed,
        )
        .seal(coord_encr_pk, &part.encr_sk);
        let message = MessageBuffer::new(&sbox, &updatebox).seal();

        Self { message, mask_seed }
    }
}

#[derive(Debug)]
/// A "sum2" message and an url for a mask of a global model.
pub struct Sum2Message {
    pub message: Vec<u8>,  // 257 bytes
    pub mask_url: Vec<u8>, // 32 bytes (dummy)
}

impl Sum2Message {
    /// Compute a global mask from all local masks and encrypt the "sum2" message parts. Eligibility
    /// for the "sum" task should be checked beforehand.
    fn compose(
        part: &Participant,
        coord_encr_pk: &box_::PublicKey,
        dict_seed: &HashMap<box_::PublicKey, HashMap<box_::PublicKey, Vec<u8>>>,
    ) -> Result<Self, PetError> {
        // compute global mask
        let mut seeds: Vec<Vec<u8>> = Vec::new();
        for seed in dict_seed
            .get(&part.encr_pk)
            .ok_or(PetError::InvalidMessage)?
            .values()
        {
            seeds.extend(vec![sealedbox::open(seed, &part.ephm_pk, &part.ephm_sk)
                .or(Err(PetError::InvalidMessage))?]);
        }
        let mask_url = randombytes(32); // dummy

        // encrypt message parts
        let sbox = SealedBoxBuffer::new(&part.encr_pk, &part.sign_pk).seal(coord_encr_pk);
        let sum2box = Sum2BoxBuffer::new(&part.certificate, &part.signature_sum, &mask_url)
            .seal(coord_encr_pk, &part.encr_sk);
        let message = MessageBuffer::new(&sbox, &sum2box).seal();

        Ok(Self { message, mask_url })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pet::coordinator::Coordinator;

    #[test]
    fn test_participant() {
        // new
        let mut part = Participant::new().unwrap();
        assert_eq!(part.encr_pk, part.encr_sk.public_key());
        assert_eq!(part.encr_sk.as_ref().len(), 32);
        assert_eq!(part.sign_pk, part.sign_sk.public_key());
        assert_eq!(part.sign_sk.as_ref().len(), 64);
        assert_eq!(part.ephm_pk, box_::PublicKey([0_u8; 32]));
        assert_eq!(part.ephm_sk, box_::SecretKey([0_u8; 32]));
        assert_eq!(part.certificate, vec![0_u8; 0]);
        assert_eq!(part.signature_sum, sign::Signature([0_u8; 64]));
        assert_eq!(part.signature_update, sign::Signature([0_u8; 64]));
        assert_eq!(part.task, Task::None);

        // compute signature
        let seed = randombytes(32);
        part.compute_signatures(&seed);
        assert_eq!(
            part.signature_sum,
            sign::sign_detached(&[seed.as_slice(), b"sum"].concat(), &part.sign_sk)
        );
        assert_eq!(
            part.signature_update,
            sign::sign_detached(&[seed.as_slice(), b"update"].concat(), &part.sign_sk)
        );

        // check task
        let sign_ell = sign::Signature([
            229, 191, 74, 163, 113, 6, 242, 191, 255, 225, 40, 89, 210, 94, 25, 50, 44, 129, 155,
            241, 99, 64, 25, 212, 157, 235, 102, 95, 115, 18, 158, 115, 253, 136, 178, 223, 4, 47,
            54, 162, 236, 78, 126, 114, 205, 217, 250, 163, 223, 149, 31, 65, 179, 179, 60, 64, 34,
            1, 78, 245, 1, 50, 165, 47,
        ]);
        let sign_inell = sign::Signature([
            15, 107, 81, 84, 105, 246, 165, 81, 76, 125, 140, 172, 113, 85, 51, 173, 119, 123, 78,
            114, 249, 182, 135, 212, 134, 38, 125, 153, 120, 45, 179, 55, 116, 155, 205, 51, 247,
            37, 78, 147, 63, 231, 28, 61, 251, 41, 48, 239, 125, 0, 129, 126, 194, 123, 183, 11,
            215, 220, 1, 225, 248, 131, 64, 242,
        ]);
        part.signature_sum = sign_ell.clone();
        part.signature_update = sign_inell.clone();
        part.check_task(0.5_f64, 0.5_f64);
        assert_eq!(part.task, Task::Sum);
        part.signature_update = sign_ell.clone();
        part.check_task(0.5_f64, 0.5_f64);
        assert_eq!(part.task, Task::Sum);
        part.signature_sum = sign_inell.clone();
        part.check_task(0.5_f64, 0.5_f64);
        assert_eq!(part.task, Task::Update);
        part.signature_update = sign_inell.clone();
        part.check_task(0.5_f64, 0.5_f64);
        assert_eq!(part.task, Task::None);
    }

    #[test]
    fn test_sealedboxbuffer() {
        let coord = Coordinator::new().unwrap();
        let part = Participant::new().unwrap();

        // new
        let buf = SealedBoxBuffer::new(&part.encr_pk, &part.sign_pk);
        assert_eq!(buf.0, b"round");
        assert_eq!(buf.1, part.encr_pk.as_ref());
        assert_eq!(buf.2, part.sign_pk.as_ref());

        // seal
        let sbox = buf.seal(&coord.encr_pk);
        assert_eq!(sbox.len(), 48 + 5 + 32 + 32);
    }

    #[test]
    fn test_sumboxbuffer() {
        let coord = Coordinator::new().unwrap();
        let part = Participant::new().unwrap();

        // new
        let buf = SumBoxBuffer::new(&part.certificate, &part.signature_sum, &part.ephm_pk);
        assert_eq!(buf.0, b"sum");
        assert_eq!(buf.1, part.certificate.as_slice());
        assert_eq!(buf.2, part.signature_sum.as_ref());
        assert_eq!(buf.3, part.ephm_pk.as_ref());

        // seal
        let sumbox = buf.seal(&coord.encr_pk, &part.encr_sk);
        assert_eq!(sumbox.len(), 24 + 16 + 3 + 0 + 64 + 32)
    }

    #[test]
    fn test_updateboxbuffer() {
        let coord = Coordinator::new().unwrap();
        let part = Participant::new().unwrap();

        // new
        let model_url = randombytes(32);
        let dict_seed = randombytes(112);
        let buf = UpdateBoxBuffer::new(
            &part.certificate,
            &part.signature_sum,
            &part.signature_update,
            &model_url,
            &dict_seed,
        );
        assert_eq!(buf.0, b"update");
        assert_eq!(buf.1, part.certificate.as_slice());
        assert_eq!(buf.2, part.signature_sum.as_ref());
        assert_eq!(buf.3, part.signature_update.as_ref());
        assert_eq!(buf.4, model_url.as_slice());
        assert_eq!(buf.5, dict_seed.as_slice());

        // seal
        let sumbox = buf.seal(&coord.encr_pk, &part.encr_sk);
        assert_eq!(sumbox.len(), 24 + 16 + 6 + 0 + 64 + 64 + 32 + 112)
    }

    #[test]
    fn test_sum2boxbuffer() {
        let coord = Coordinator::new().unwrap();
        let part = Participant::new().unwrap();

        // new
        let mask_url = randombytes(32);
        let buf = Sum2BoxBuffer::new(&part.certificate, &part.signature_sum, &mask_url);
        assert_eq!(buf.0, b"sum2");
        assert_eq!(buf.1, part.certificate.as_slice());
        assert_eq!(buf.2, part.signature_sum.as_ref());
        assert_eq!(buf.3, mask_url.as_slice());

        // seal
        let sumbox = buf.seal(&coord.encr_pk, &part.encr_sk);
        assert_eq!(sumbox.len(), 24 + 16 + 4 + 0 + 64 + 32)
    }

    #[test]
    fn test_messagebuffer() {
        // new
        let sbox = randombytes(48);
        let box__ = randombytes(40);
        let buf = MessageBuffer::new(&sbox, &box__);
        assert_eq!(buf.0, sbox.as_slice());
        assert_eq!(buf.1, box__.as_slice());

        // seal
        let msg = buf.seal();
        assert_eq!(msg.len(), 48 + 40);
        assert_eq!(msg[0..48].to_vec(), sbox);
        assert_eq!(msg[48..88].to_vec(), box__);
    }

    #[test]
    fn test_summessage() {
        let coord = Coordinator::new().unwrap();
        let mut part = Participant::new().unwrap();
        let zpk = box_::PublicKey([0_u8; 32]);
        let zsk = box_::SecretKey([0_u8; 32]);

        // compose
        assert_eq!(part.ephm_pk, zpk);
        assert_eq!(part.ephm_sk, zsk);
        let msg = SumMessage::compose(&mut part, &coord.encr_pk);
        assert_eq!(msg.message.len(), 117 + 139);
        assert_ne!(part.ephm_pk, zpk);
        assert_ne!(part.ephm_sk, zsk);
    }

    #[test]
    fn test_updatemessage() {
        let coord = Coordinator::new().unwrap();
        let part = Participant::new().unwrap();
        let encr_pk = box_::PublicKey::from_slice(&randombytes(32)).unwrap();
        let ephm_pk = box_::PublicKey::from_slice(&randombytes(32)).unwrap();
        let dict_sum = [(encr_pk, ephm_pk)].iter().cloned().collect();

        // compose
        let msg = UpdateMessage::compose(&part, &coord.encr_pk, &dict_sum);
        assert_eq!(msg.message.len(), 117 + 206 + 112);
        assert_eq!(msg.mask_seed.len(), 32);
    }

    #[test]
    fn test_sum2message() {
        let coord = Coordinator::new().unwrap();
        let mut part = Participant::new().unwrap();
        let (ephm_pk, ephm_sk) = box_::gen_keypair();
        part.ephm_pk = ephm_pk;
        part.ephm_sk = ephm_sk;
        let mut dict_seed = [(
            part.encr_pk,
            [(
                box_::PublicKey::from_slice(&randombytes(32)).unwrap(),
                sealedbox::seal(&randombytes(32), &part.ephm_pk),
            )]
            .iter()
            .cloned()
            .collect(),
        )]
        .iter()
        .cloned()
        .collect();

        // compose
        let msg = Sum2Message::compose(&part, &coord.encr_pk, &dict_seed).unwrap();
        assert_eq!(msg.message.len(), 117 + 140);
        assert_eq!(msg.mask_url.len(), 32);

        // compose error: missing participant key
        dict_seed.clear();
        assert_eq!(
            Sum2Message::compose(&part, &coord.encr_pk, &dict_seed).unwrap_err(),
            PetError::InvalidMessage
        );

        // compose error: failing seed decryption
        dict_seed.insert(
            part.encr_pk,
            [(
                box_::PublicKey::from_slice(&randombytes(32)).unwrap(),
                randombytes(80),
            )]
            .iter()
            .cloned()
            .collect(),
        );
        assert_eq!(
            Sum2Message::compose(&part, &coord.encr_pk, &dict_seed).unwrap_err(),
            PetError::InvalidMessage
        );
    }
}
