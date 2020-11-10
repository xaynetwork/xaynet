use std::convert::TryFrom;

use derive_more::{From, Into};
use paste::paste;
use redis::{ErrorKind, FromRedisValue, RedisError, RedisResult, RedisWrite, ToRedisArgs, Value};
use serde::{Deserialize, Serialize};

use crate::{
    state_machine::coordinator::CoordinatorState,
    storage::{
        LocalSeedDictAdd,
        LocalSeedDictAddError,
        MaskScoreIncr,
        MaskScoreIncrError,
        SumPartAdd,
        SumPartAddError,
    },
};
use xaynet_core::{
    crypto::{ByteObject, PublicEncryptKey, PublicSigningKey},
    mask::{EncryptedMaskSeed, MaskObject},
    LocalSeedDict,
};

fn redis_type_error(desc: &'static str, details: Option<String>) -> RedisError {
    if let Some(details) = details {
        RedisError::from((ErrorKind::TypeError, desc, details))
    } else {
        RedisError::from((ErrorKind::TypeError, desc))
    }
}

fn error_code_type_error(response: &Value) -> RedisError {
    redis_type_error(
        "Response status not valid integer",
        Some(format!("Response was {:?}", response)),
    )
}

/// Implements ['FromRedisValue'] and ['ToRedisArgs'] for types that implement ['ByteObject'].
/// The Redis traits as well as the crypto types are both defined in foreign crates.
/// To bypass the restrictions of orphan rule, we use `Newtypes` for the crypto types.
///
/// Each crypto type has two `Newtypes`, one for reading and one for writing.
/// The difference between `Read` and `Write` is that the write `Newtype` does not take the
/// ownership of the value but only a reference. This allows us to use references in the
/// [`Client`] methods. The `Read` Newtype also implements [`ToRedisArgs`] to reduce the
/// conversion overhead that you would get if you wanted to reuse a `Read` value for another
/// Redis query.
///
/// Example:
///
/// ```ignore
/// let sum_pks: Vec<PublicSigningKeyRead> = self.connection.hkeys("sum_dict").await?;
/// for sum_pk in sum_pks {
///    let sum_pk_seed_dict: HashMap<PublicSigningKeyRead, EncryptedMaskSeedRead>
///       = self.connection.hgetall(&sum_pk).await?; // no need to convert sum_pk from PublicSigningKeyRead to PublicSigningKeyWrite
/// }
/// ```
///
/// [`Client`]: crate::storage::redis::Client
macro_rules! impl_byte_object_redis_traits {
    ($ty: ty) => {
        paste! {
            #[derive(Into, Hash, Eq, PartialEq)]
            pub(crate) struct [<$ty Read>]($ty);

            impl FromRedisValue for [<$ty Read>] {
                fn from_redis_value(v: &Value) -> RedisResult<[<$ty Read>]> {
                    match *v {
                        Value::Data(ref bytes) => {
                            let inner = <$ty>::from_slice(bytes).ok_or_else(|| {
                                redis_type_error(concat!("Invalid ", stringify!($ty)), None)
                            })?;
                            Ok([<$ty Read>](inner))
                        }
                        _ => Err(redis_type_error(
                            concat!("Response not ", stringify!($ty), " compatible"),
                            None,
                        )),
                    }
                }
            }

            impl ToRedisArgs for [<$ty Read>] {
                fn write_redis_args<W>(&self, out: &mut W)
                where
                    W: ?Sized + RedisWrite,
                {
                    self.0.as_slice().write_redis_args(out)
                }
            }

            impl<'a> ToRedisArgs for &'a [<$ty Read>] {
                fn write_redis_args<W>(&self, out: &mut W)
                where
                    W: ?Sized + RedisWrite,
                {
                    self.0.as_slice().write_redis_args(out)
                }
            }

            #[derive(From)]
            pub(crate) struct [<$ty Write>]<'a>(&'a $ty);

            impl ToRedisArgs for [<$ty Write>]<'_> {
                fn write_redis_args<W>(&self, out: &mut W)
                where
                    W: ?Sized + RedisWrite,
                {
                    self.0.as_slice().write_redis_args(out)
                }
            }

            impl<'a> ToRedisArgs for &'a [<$ty Write>]<'a> {
                fn write_redis_args<W>(&self, out: &mut W)
                where
                    W: ?Sized + RedisWrite,
                {
                    self.0.as_slice().write_redis_args(out)
                }
            }
        }
    };
}

impl_byte_object_redis_traits!(PublicEncryptKey);
impl_byte_object_redis_traits!(PublicSigningKey);
impl_byte_object_redis_traits!(EncryptedMaskSeed);

/// Implements ['FromRedisValue'] and ['ToRedisArgs'] for types that implement
/// ['Serialize`] and [`Deserialize']. The data is de/serialized via bincode.
///
/// # Panics
///
/// `write_redis_args` will panic if the data cannot be serialized with `bincode`
///
/// More information about what can cause a panic in bincode:
/// - https://github.com/servo/bincode/issues/293
/// - https://github.com/servo/bincode/issues/255
/// - https://github.com/servo/bincode/issues/130#issuecomment-284641263
macro_rules! impl_bincode_redis_traits {
    ($ty: ty) => {
        impl FromRedisValue for $ty {
            fn from_redis_value(v: &Value) -> RedisResult<$ty> {
                match *v {
                    Value::Data(ref bytes) => bincode::deserialize(bytes)
                        .map_err(|e| redis_type_error("Invalid data", Some(e.to_string()))),
                    _ => Err(redis_type_error("Response not bincode compatible", None)),
                }
            }
        }

        impl ToRedisArgs for $ty {
            fn write_redis_args<W>(&self, out: &mut W)
            where
                W: ?Sized + RedisWrite,
            {
                let data = bincode::serialize(self).unwrap();
                data.write_redis_args(out)
            }
        }

        impl<'a> ToRedisArgs for &'a $ty {
            fn write_redis_args<W>(&self, out: &mut W)
            where
                W: ?Sized + RedisWrite,
            {
                (*self).write_redis_args(out)
            }
        }
    };
}

// CoordinatorState is pretty straightforward:
// - all the sequences have known length (
// - no untagged enum
// so bincode will not panic.
impl_bincode_redis_traits!(CoordinatorState);

#[derive(From, Into, Serialize, Deserialize)]
pub(crate) struct MaskObjectRead(MaskObject);

impl_bincode_redis_traits!(MaskObjectRead);

#[derive(From, Serialize)]
pub(crate) struct MaskObjectWrite<'a>(&'a MaskObject);

impl ToRedisArgs for MaskObjectWrite<'_> {
    fn write_redis_args<W>(&self, out: &mut W)
    where
        W: ?Sized + RedisWrite,
    {
        let data = bincode::serialize(self).unwrap();
        data.write_redis_args(out)
    }
}

impl<'a> ToRedisArgs for &'a MaskObjectWrite<'a> {
    fn write_redis_args<W>(&self, out: &mut W)
    where
        W: ?Sized + RedisWrite,
    {
        (*self).write_redis_args(out)
    }
}

#[derive(From)]
pub(crate) struct LocalSeedDictWrite<'a>(&'a LocalSeedDict);

impl ToRedisArgs for LocalSeedDictWrite<'_> {
    fn write_redis_args<W>(&self, out: &mut W)
    where
        W: ?Sized + RedisWrite,
    {
        let args: Vec<(PublicSigningKeyWrite, EncryptedMaskSeedWrite)> = self
            .0
            .iter()
            .map(|(pk, seed)| {
                (
                    PublicSigningKeyWrite::from(pk),
                    EncryptedMaskSeedWrite::from(seed),
                )
            })
            .collect();

        args.write_redis_args(out)
    }
}

impl<'a> ToRedisArgs for &'a LocalSeedDictWrite<'a> {
    fn write_redis_args<W>(&self, out: &mut W)
    where
        W: ?Sized + RedisWrite,
    {
        let args: Vec<(PublicSigningKeyWrite, EncryptedMaskSeedWrite)> = self
            .0
            .iter()
            .map(|(pk, seed)| {
                (
                    PublicSigningKeyWrite::from(pk),
                    EncryptedMaskSeedWrite::from(seed),
                )
            })
            .collect();

        args.write_redis_args(out)
    }
}

impl FromRedisValue for LocalSeedDictAdd {
    fn from_redis_value(v: &Value) -> RedisResult<LocalSeedDictAdd> {
        match *v {
            Value::Int(0) => Ok(LocalSeedDictAdd(Ok(()))),
            Value::Int(error_code) => match LocalSeedDictAddError::try_from(error_code) {
                Ok(error_variant) => Ok(LocalSeedDictAdd(Err(error_variant))),
                Err(_) => Err(error_code_type_error(v)),
            },
            _ => Err(error_code_type_error(v)),
        }
    }
}

impl FromRedisValue for SumPartAdd {
    fn from_redis_value(v: &Value) -> RedisResult<SumPartAdd> {
        match *v {
            Value::Int(1) => Ok(SumPartAdd(Ok(()))),
            Value::Int(error_code) => match SumPartAddError::try_from(error_code) {
                Ok(error_variant) => Ok(SumPartAdd(Err(error_variant))),
                Err(_) => Err(error_code_type_error(v)),
            },
            _ => Err(error_code_type_error(v)),
        }
    }
}

impl FromRedisValue for MaskScoreIncr {
    fn from_redis_value(v: &Value) -> RedisResult<MaskScoreIncr> {
        match *v {
            Value::Int(0) => Ok(MaskScoreIncr(Ok(()))),
            Value::Int(error_code) => match MaskScoreIncrError::try_from(error_code) {
                Ok(error_variant) => Ok(MaskScoreIncr(Err(error_variant))),
                Err(_) => Err(error_code_type_error(v)),
            },
            _ => Err(error_code_type_error(v)),
        }
    }
}

#[cfg(test)]
#[derive(derive_more::Deref)]
pub struct SumDictDelete(Result<(), SumDictDeleteError>);

#[cfg(test)]
impl SumDictDelete {
    pub fn into_inner(self) -> Result<(), SumDictDeleteError> {
        self.0
    }
}

#[cfg(test)]
#[derive(thiserror::Error, Debug, num_enum::TryFromPrimitive)]
#[repr(i64)]
pub enum SumDictDeleteError {
    #[error("sum participant does not exist")]
    DoesNotExist = 0,
}

#[cfg(test)]
impl FromRedisValue for SumDictDelete {
    fn from_redis_value(v: &Value) -> RedisResult<SumDictDelete> {
        match *v {
            Value::Int(1) => Ok(SumDictDelete(Ok(()))),
            Value::Int(error_code) => match SumDictDeleteError::try_from(error_code) {
                Ok(error_variant) => Ok(SumDictDelete(Err(error_variant))),
                Err(_) => Err(error_code_type_error(v)),
            },
            _ => Err(error_code_type_error(v)),
        }
    }
}
