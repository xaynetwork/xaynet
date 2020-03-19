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
pub fn compose_sum_message(
    coord_encr_pk: &box_::PublicKey,
    part_encr_pk: &box_::PublicKey,
    part_encr_sk: &box_::SecretKey,
    part_sign_pk: &sign::PublicKey,
) -> Result<(box_::PublicKey, box_::SecretKey, [Vec<u8>; 5]), ()> {
    // initialize csprng
    let _ = init()?;

    // generate ephemeral key pair
    (part_ephm_pk, part_ephm_sk) = box_::gen_keypair();

    // encrypt message parts
    key = box_::precompute(&coord_encr_pk, &part_encr_sk);
    nonce = [
        box_::gen_nonce(),
        box_::gen_nonce(),
        box_::gen_nonce(),
        box_::gen_nonce(),
    ];
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
        [
            nonce[0].0.to_vec(),
            box_::seal_precomputed(b"sum", &nonce[0], &key),
        ]
        .concat(),
        // dummy certificate: 40 + 0 bytes
        [
            nonce[1].0.to_vec(),
            box_::seal_precomputed(b"", &nonce[1], &key),
        ]
        .concat(),
        // ephemeral key: 40 + 32 bytes
        [
            nonce[2].0.to_vec(),
            box_::seal_precomputed(&part_ephm_pk.0, &nonce[2], &key),
        ]
        .concat(),
        // dummy signature: 40 + 0 bytes
        [
            nonce[3].0.to_vec(),
            box_::seal_precomputed(b"", &nonce[3], &key),
        ]
        .concat(),
    ];

    (part_ephm_pk, part_ephm_sk, message)
}
