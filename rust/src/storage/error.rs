use thiserror::Error;

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("Read error!")]
    Read,
    #[error("Convert error!")]
    Convert,
}
