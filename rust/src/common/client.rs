use derive_more::{Display, From};
use std::str::FromStr;
use uuid::{self, Uuid};

#[derive(
    Eq, PartialEq, Hash, Debug, Copy, Clone, Display, Serialize, Deserialize, Default, From,
)]
/// A unique random client identifier
pub struct ClientId(Uuid);

impl ClientId {
    /// Return a new random client identifier
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl FromStr for ClientId {
    type Err = uuid::Error;
    fn from_str(uuid_str: &str) -> Result<Self, Self::Err> {
        Ok(Self(Uuid::from_str(uuid_str)?))
    }
}

#[derive(
    Eq, PartialEq, Hash, Debug, Copy, Clone, Display, Serialize, Deserialize, Default, From,
)]
/// A unique random token
pub struct Token(Uuid);

impl FromStr for Token {
    type Err = uuid::Error;
    fn from_str(uuid_str: &str) -> Result<Self, Self::Err> {
        Ok(Self(Uuid::from_str(uuid_str)?))
    }
}

impl Token {
    /// Return a new random token
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

#[derive(Eq, PartialEq, Hash, Debug, Copy, Clone, Serialize, Deserialize, Default, From)]
pub struct Credentials(pub ClientId, pub Token);

impl Credentials {
    pub fn id(&self) -> &ClientId {
        &self.0
    }
    pub fn token(&self) -> &Token {
        &self.1
    }
    pub fn into_parts(self) -> (ClientId, Token) {
        (self.0, self.1)
    }
}
