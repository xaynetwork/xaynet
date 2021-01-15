//! Storage backends to manage global models.

pub mod noop;
#[cfg(feature = "model-persistence")]
#[cfg_attr(docsrs, doc(cfg(feature = "model-persistence")))]
pub mod s3;
