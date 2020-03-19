use sodiumoxide::{box_, init, sealedbox, sign};
use std::collections::HashMap;

/// # Validate the received "sum" message.
/// Decrypt and validate the message parts from a "sum" participant. Then update the
/// dictionary of "sum" participants.
///
/// ## Note
/// Corresponds to steps 5., 6. and 7. of the PET protocol.
///
/// ## Args
/// - `coord_encr_pk`: The public key for asymmetric encryption of the coordinator.
/// - `coord_encr_sk`: The private key for asymmetric encryption of the coordinator.
/// - `message`: An encrypted message from a participant.
/// - `dict_sum`: The dictionary of "sum" participants.
///
/// ## Raises
/// - Err(_): If validation fails.
pub fn validate_sum_message(
    coord_encr_pk: &box_::PublicKey,
    coord_encr_sk: &box_::SecretKey,
    message: &[Vec<u8>; 5],
    dict_sum: &mut HashMap<box_::PublicKey, box_::PublicKey>,
) -> Result<(), ()> {
    // validate "round" message and get participant public keys
    let msg = sealedbox::open(&message[0], &coord_encr_pk, &coord_encr_sk)?;
    if msg[64..] != b"round" {
        return Err(());
    }
    let sum_encr_pk = box_::PublicKey(&msg[..32]);
    let _sum_sign_pk = sign::PublicKey(&msg[32..64]);

    // compute shared symmetric key
    let key = box_::precompute(&sum_encr_pk, &coord_encr_sk);

    // validate "sum" message
    let msg = box_::open_precomputed(&message[1][24..], box_::Nonce(&message[1][..24]), &key);
    if msg != b"sum" {
        return Err(());
    }

    // validate dummy certificate
    let msg = box_::open_precomputed(&message[2][24..], box_::Nonce(&message[2][..24]), &key);
    if msg != b"" {
        return Err(());
    }

    // get participant ephemeral public key
    let msg = box_::open_precomputed(&message[3][24..], box_::Nonce(&message[3][..24]), &key);
    let sum_ephm_pk = box_::PublicKey(&msg);

    // validate dummy "sum" signature
    let msg = box_::open_precomputed(&message[4][24..], box_::Nonce(&message[4][..24]), &key);
    if msg != b"" {
        return Err(());
    }

    // update dictionary of "sum" participants
    dict_sum.insert(sum_encr_pk, sum_ephm_pk);
    Ok(())
}
