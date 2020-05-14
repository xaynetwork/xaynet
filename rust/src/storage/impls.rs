use crate::{
    coordinator::{CoordinatorState, RoundSeed},
    crypto::{self, ByteObject},
    mask::seed::EncryptedMaskSeed,
};
use redis::{ErrorKind, FromRedisValue, RedisError, RedisResult, RedisWrite, ToRedisArgs, Value};

fn redis_type_error(desc: &'static str, details: Option<String>) -> RedisError {
    if let Some(details) = details {
        RedisError::from((ErrorKind::TypeError, desc, details))
    } else {
        RedisError::from((ErrorKind::TypeError, desc))
    }
}

macro_rules! impl_redis_traits {
    ($ty: ty) => {
        impl FromRedisValue for $ty {
            fn from_redis_value(v: &Value) -> RedisResult<$ty> {
                match *v {
                    Value::Data(ref bytes) => <$ty>::from_slice(bytes).ok_or_else(|| {
                        redis_type_error(concat!("Invalid ", stringify!($ty)), None)
                    }),
                    _ => Err(redis_type_error(
                        concat!("Response not ", stringify!($ty), " compatible"),
                        None,
                    )),
                }
            }
        }

        impl ToRedisArgs for $ty {
            fn write_redis_args<W>(&self, out: &mut W)
            where
                W: ?Sized + RedisWrite,
            {
                self.as_slice().write_redis_args(out)
            }
        }

        impl<'a> ToRedisArgs for &'a $ty {
            fn write_redis_args<W>(&self, out: &mut W)
            where
                W: ?Sized + RedisWrite,
            {
                self.as_slice().write_redis_args(out)
            }
        }
    };
}

impl_redis_traits!(crypto::PublicEncryptKey);
impl_redis_traits!(crypto::SecretEncryptKey);
impl_redis_traits!(crypto::PublicSigningKey);
impl_redis_traits!(crypto::SecretSigningKey);
impl_redis_traits!(crypto::Signature);
impl_redis_traits!(EncryptedMaskSeed);
impl_redis_traits!(RoundSeed);

impl FromRedisValue for CoordinatorState {
    fn from_redis_value(v: &Value) -> RedisResult<CoordinatorState> {
        match *v {
            Value::Data(ref bytes) => bincode::deserialize(bytes)
                .map_err(|e| redis_type_error("Invalid CoordinatorState", Some(e.to_string()))),
            _ => Err(redis_type_error(
                "Response not CoordinatorState compatible",
                None,
            )),
        }
    }
}

impl ToRedisArgs for CoordinatorState {
    fn write_redis_args<W>(&self, out: &mut W)
    where
        W: ?Sized + RedisWrite,
    {
        // CoordinatorState is pretty straighforward:
        // - all the sequences have known length (
        // - no untagged enum
        // so it is safe to unwrap here.
        //
        // Refs:
        // - https://github.com/servo/bincode/issues/293
        // - https://github.com/servo/bincode/issues/255
        // - https://github.com/servo/bincode/issues/130#issuecomment-284641263
        let data = bincode::serialize(self).unwrap();
        data.write_redis_args(out)
    }
}

impl<'a> ToRedisArgs for &'a CoordinatorState {
    fn write_redis_args<W>(&self, out: &mut W)
    where
        W: ?Sized + RedisWrite,
    {
        (*self).write_redis_args(out)
    }
}
