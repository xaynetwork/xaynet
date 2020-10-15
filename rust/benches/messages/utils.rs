pub mod update {
    use xaynet_core::{
        message::{ToBytes, Update},
        testutils::multipart,
    };

    fn make_update(
        dict_len: usize,
        mask_len: usize,
        total_expected_len: usize,
    ) -> (Update, Vec<u8>) {
        let update = multipart::update(dict_len, mask_len);
        // just check that we made our calculation right
        // message size = dict_len + mask_len + 64*2
        assert_eq!(update.buffer_length(), total_expected_len);
        let mut bytes = vec![0; update.buffer_length()];
        update.to_bytes(&mut bytes);
        (update, bytes)
    }

    /// Get an update that corresponds to:
    ///     - 1 sum participant (1 entry in the seed dict)
    ///     - a 128 bytes serialized masked model
    pub fn update_tiny() -> (Update, Vec<u8>) {
        // 116 + 28 + 128 = 272
        make_update(116, 28, 272)
    }

    /// Get an update that corresponds to:
    ///     - 1k sum participants (1k entries in the seed dict)
    ///     - a 6kB serialized masked model
    pub fn update_100kB() -> (Update, Vec<u8>) {
        // message size = 112004 + 6022 + 128 = 118_154
        make_update(112_004, 6_022, 118_154)
    }

    /// Get an update that corresponds to:
    ///     - 10k sum participants (10k entries in the seed dict)
    ///     - a 60kB serialized masked model
    pub fn update_1MB() -> (Update, Vec<u8>) {
        // 1120004 + 60022 + 128 = 1_180_154
        make_update(1_120_004, 60_022, 1_180_154)
    }

    /// Get an update that corresponds to:
    ///     - 10k sum participants (10k entries in the seed dict)
    ///     - a ~1MB serialized masked model
    pub fn update_2MB() -> (Update, Vec<u8>) {
        // 1120004 + 1000024 + 128 = 2_120_156
        make_update(1_120_004, 1_000_024, 2_120_156)
    }

    /// Get an update that corresponds to:
    ///     - 10k sum participants (10k entries in the seed dict)
    ///     - a ~9MB serialized masked model
    pub fn update_10MB() -> (Update, Vec<u8>) {
        // 1120004 + 9000026 + 128 = 10_120_154
        make_update(1_120_004, 9_000_022, 10_120_154)
    }
}
