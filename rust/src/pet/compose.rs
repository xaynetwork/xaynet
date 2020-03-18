use sodiumoxide::{box_, init, sealedbox, sign};

/// # Construct the "sum" message.
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
/// The ephemeral key pair and the encrypted "sum" message as `Ok(_)`.
///
/// ## Raises
/// - `Err(_)`: If initialization of the CSPRNG fails.
pub fn sum_message(
    coord_encr_pk: &box_::PublicKey,
    part_encr_pk: &box_::PublicKey,
    part_encr_sk: &box_::SecretKey,
    part_sign_pk: &sign::PublicKey,
) -> Result<(box_::PublicKey, box_::SecretKey, [Vec<u8>; 5]), ()> {
    // initialize csprng
    if let Err(e) = init() {
        return Err(e);
    }

    // generate ephemeral key pair
    (part_ephm_pk, part_ephm_sk) = box_::gen_keypair();

    // encrypt message parts
    key = box_::precompute(&coord_encr_pk, &part_encr_sk);
    let message: [Vec<u8>; 5] = [
        // public keys & "round": 48 + 69 bytes
        sealedbox::seal(
            &[
                part_encr_pk.0.to_vec(),
                part_sign_pk.0.to_vec(),
                b"round".to_vec(),
            ]
            .concat(),
            &coord_encr_pk,
        ),
        // "sum": 40 + 3 bytes
        box_::seal_precomputed(&b"sum", &box_::gen_nonce(), &key),
        // dummy certificate: 40 + 0 bytes
        box_::seal_precomputed(&b"", &box_::gen_nonce(), &key),
        // ephemeral key: 40 + 32 bytes
        box_::seal_precomputed(&part_ephm_pk.0.to_vec(), &box_::gen_nonce(), &key),
        // dummy signature: 40 + 0 bytes
        box_::seal_precomputed(&b"", &box_::gen_nonce(), &key),
    ];

    (part_ephm_pk, part_ephm_sk, message)
}
