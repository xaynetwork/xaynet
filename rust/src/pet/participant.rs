use sodiumoxide::{
    crypto::{box_, sealedbox, sign},
    init,
    randombytes::randombytes,
};
use std::collections::HashMap;

/// # Compose the "sum" message.
/// Generate an ephemeral asymmetric key pair and encrypt the message parts. Eligibility
/// for the "sum" task should be checked beforehand.
///
/// ## Note
/// Corresponds to step 4. of the PET protocol.
///
/// ## Args
/// - `coord_encr_pk`: The public key for asymmetric encryption of the coordinator.
/// - `part_encr_pk`: The public key for asymmetric encryption of the participant.
/// - `part_encr_sk`: The private key for asymmetric encryption of the participant.
/// - `part_sign_pk`: The public key for signatures of the participant.
///
/// ## Returns
/// The ephemeral key pair and the encrypted "sum" message as `Ok((pk, sk, m))`.
///
/// ## Raises
/// - `Err(())`: If initialization of the CSPRNG fails.
pub fn compose_sum_message(
    coord_encr_pk: &box_::PublicKey,
    part_encr_pk: &box_::PublicKey,
    part_encr_sk: &box_::SecretKey,
    part_sign_pk: &sign::PublicKey,
) -> Result<(box_::PublicKey, box_::SecretKey, Vec<u8>), ()> {
    // initialize csprng
    init()?;

    // generate ephemeral key pair
    let (part_ephm_pk, part_ephm_sk) = box_::gen_keypair();

    // encrypt message parts
    let nonce = box_::gen_nonce();
    let message = [
        sealedbox::seal(
            // 48 bytes +
            &[
                part_encr_pk.0.to_vec(), // 32 bytes
                part_sign_pk.0.to_vec(), // 32 bytes
                b"round".to_vec(),       // 5 bytes
            ]
            .concat(),
            &coord_encr_pk,
        ),
        nonce.0.to_vec(), // 24 bytes
        box_::seal(
            // 16 bytes +
            &[
                b"sum".to_vec(),         // 3 bytes
                b"".to_vec(),            // 0 bytes (dummy)
                part_ephm_pk.0.to_vec(), // 32 bytes
                b"".to_vec(),            // 0 bytes (dummy)
            ]
            .concat(),
            &nonce,
            &coord_encr_pk,
            &part_encr_sk,
        ),
    ]
    .concat(); // 192 bytes in total

    Ok((part_ephm_pk, part_ephm_sk, message))
}

// /// # Compose the "update" message.
// /// Mask a trained local model, create a dictionary of encrypted masking seeds and
// /// encrypt the message parts. Eligibility for the "update" task should be checked
// /// beforehand.
// ///
// /// ## Note
// /// Corresponds to step 9. of the PET protocol.
// ///
// /// ## Args
// /// - broadcast: The broadcasted state.
// ///
// /// ## Returns
// /// The encrypted "update" message.
// ///
// /// ## Raises
// /// - PetProtocolError: If broadcast parameters are missing or invalid.
// pub fn compose_update_message(
//     dict_sum: &HashMap<box_::PublicKey, box_::PublicKey>,
// ) -> Result<(), ()> {
//     // initialize csprng
//     init()?;

//     // mask the local model
//     seed = randombytes(32);
//     url_masked_local_model = randombytes(32); // dummy

//     // create dictionary of encrypted masking seeds

//     // encrypt message parts
// }
