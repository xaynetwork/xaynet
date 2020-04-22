pub mod serde_counter {
    use crate::coordinator::MaskHash;
    use counter::Counter;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    #[derive(Serialize, Deserialize)]
    struct CounterHelper {
        keys: Vec<MaskHash>,
    }

    pub fn serialize<S>(counter: &Counter<MaskHash>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        CounterHelper {
            keys: counter.keys().map(|k| k.clone()).collect(),
        }
        .serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Counter<MaskHash>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let counter = CounterHelper::deserialize(deserializer)?;
        Ok(Counter::init(counter.keys))
    }
}

pub mod serde_sodiumoxide {
    use core::iter::Map;
    use data_encoding::HEXUPPER;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use sodiumoxide::crypto::{box_, sign};
    use std::collections::HashMap;

    pub fn u8vec_as_hex<T, S>(data: &T, serializer: S) -> Result<S::Ok, S::Error>
    where
        T: AsRef<[u8]>,
        S: Serializer,
    {
        serializer.serialize_str(&HEXUPPER.encode(&data.as_ref()))
    }

    pub fn enc_pubkey_from_hex<'de, D>(deserializer: D) -> Result<box_::PublicKey, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::Error;
        String::deserialize(deserializer)
            .and_then(|string| {
                HEXUPPER
                    .decode(&string.as_bytes())
                    .map_err(|err| Error::custom(err.to_string()))
            })
            .map(|bytes| box_::PublicKey::from_slice(&bytes))
            .and_then(|opt| opt.ok_or_else(|| Error::custom("failed to deserialize public key")))
    }

    pub fn enc_seckey_from_hex<'de, D>(deserializer: D) -> Result<box_::SecretKey, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::Error;
        String::deserialize(deserializer)
            .and_then(|string| {
                HEXUPPER
                    .decode(&string.as_bytes())
                    .map_err(|err| Error::custom(err.to_string()))
            })
            .map(|bytes| box_::SecretKey::from_slice(&bytes))
            .and_then(|opt| opt.ok_or_else(|| Error::custom("failed to deserialize public key")))
    }

    pub fn sign_pubkey_from_hex<'de, D>(deserializer: D) -> Result<sign::PublicKey, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::Error;
        String::deserialize(deserializer)
            .and_then(|string| {
                HEXUPPER
                    .decode(&string.as_bytes())
                    .map_err(|err| Error::custom(err.to_string()))
            })
            .map(|bytes| sign::PublicKey::from_slice(&bytes))
            .and_then(|opt| opt.ok_or_else(|| Error::custom("failed to deserialize public key")))
    }

    pub fn sign_seckey_from_hex<'de, D>(deserializer: D) -> Result<sign::SecretKey, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::Error;
        String::deserialize(deserializer)
            .and_then(|string| {
                HEXUPPER
                    .decode(&string.as_bytes())
                    .map_err(|err| Error::custom(err.to_string()))
            })
            .map(|bytes| sign::SecretKey::from_slice(&bytes))
            .and_then(|opt| opt.ok_or_else(|| Error::custom("failed to deserialize public key")))
    }

    pub fn se_sum_dict<S>(
        sum_dict: &HashMap<box_::PublicKey, box_::PublicKey>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        #[derive(Serialize)]
        struct Wrapper<'a>(#[serde(serialize_with = "u8vec_as_hex")] &'a box_::PublicKey);

        let map = sum_dict.iter().map(|(k, v)| (Wrapper(k), Wrapper(v)));
        serializer.collect_map(map)
    }

    pub fn de_sum_dict<'de, D>(
        deserializer: D,
    ) -> Result<HashMap<box_::PublicKey, box_::PublicKey>, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize, Hash, Eq, PartialEq)]
        struct Wrapper(#[serde(deserialize_with = "enc_pubkey_from_hex")] box_::PublicKey);

        let v = HashMap::<Wrapper, Wrapper>::deserialize(deserializer)?;
        Ok(v.into_iter()
            .map(|(Wrapper(k), Wrapper(v))| (k, v))
            .collect())
    }

    pub fn se_sub_seed_dict<S>(
        sub_seed_dict: &HashMap<box_::PublicKey, Vec<u8>>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        #[derive(Serialize)]
        struct Wrapper<'a>(#[serde(serialize_with = "u8vec_as_hex")] &'a box_::PublicKey);

        let map = sub_seed_dict.iter().map(|(k, v)| (Wrapper(k), v));
        serializer.collect_map(map)
    }

    pub fn de_sub_seed_dict<'de, D>(
        deserializer: D,
    ) -> Result<HashMap<box_::PublicKey, Vec<u8>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize, Hash, Eq, PartialEq)]
        struct Wrapper(#[serde(deserialize_with = "enc_pubkey_from_hex")] box_::PublicKey);

        let v = HashMap::<Wrapper, Vec<u8>>::deserialize(deserializer)?;
        Ok(v.into_iter().map(|(Wrapper(k), v)| (k, v)).collect())
    }

    pub fn se_seed_dict<S>(
        seed_dict: &HashMap<box_::PublicKey, HashMap<box_::PublicKey, Vec<u8>>>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        #[derive(Serialize)]
        struct Wrapper<'a>(#[serde(serialize_with = "u8vec_as_hex")] &'a box_::PublicKey);
        #[derive(Serialize)]
        struct WrapperSub<'a>(
            #[serde(serialize_with = "se_sub_seed_dict")] &'a HashMap<box_::PublicKey, Vec<u8>>,
        );

        let map = seed_dict.iter().map(|(k, v)| (Wrapper(k), WrapperSub(v)));
        serializer.collect_map(map)
    }

    pub fn de_seed_dict<'de, D>(
        deserializer: D,
    ) -> Result<HashMap<box_::PublicKey, HashMap<box_::PublicKey, Vec<u8>>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize, Hash, Eq, PartialEq)]
        struct Wrapper(#[serde(deserialize_with = "enc_pubkey_from_hex")] box_::PublicKey);

        #[derive(Deserialize)]
        struct WrapperSub(
            #[serde(deserialize_with = "de_sub_seed_dict")] HashMap<box_::PublicKey, Vec<u8>>,
        );

        let v = HashMap::<Wrapper, WrapperSub>::deserialize(deserializer)?;
        Ok(v.into_iter()
            .map(|(Wrapper(k), WrapperSub(v))| (k, v))
            .collect())
    }
}
