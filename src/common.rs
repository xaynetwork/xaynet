use derive_more::Display;
use uuid::Uuid;

#[derive(Eq, PartialEq, Hash, Debug, Copy, Clone, Display, Serialize, Deserialize, Default)]
/// A unique random client identifier
pub struct ClientId(Uuid);

impl ClientId {
    /// Return a new random client identifier
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

#[derive(Eq, PartialEq, Hash, Debug, Copy, Clone, Display, Serialize, Deserialize, Default)]
/// A unique random token
pub struct Token(Uuid);

impl Token {
    /// Return a new random token
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}
