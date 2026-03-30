use serde::{Deserialize, Serialize};
use std::fmt;

const KEY_COUNT: usize = 6;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Keymap {
    keys: [String; KEY_COUNT],
}

impl Keymap {
    pub fn parse(input: &str) -> Result<Self, KeymapParseError> {
        let keys: Vec<String> = input.split_whitespace().map(str::to_string).collect();

        if keys.len() != KEY_COUNT {
            return Err(KeymapParseError::InvalidKeyCount {
                expected: KEY_COUNT,
                found: keys.len(),
            });
        }

        Ok(Self {
            keys: keys
                .try_into()
                .expect("key count is validated before conversion"),
        })
    }

    pub fn as_string(&self) -> String {
        self.keys.join(" ")
    }

    pub fn keys(&self) -> &[String] {
        &self.keys
    }
}

impl Default for Keymap {
    fn default() -> Self {
        Self {
            keys: ["S", "D", "F", "J", "K", "L"].map(|key| key.to_string()),
        }
    }
}

impl From<Keymap> for String {
    fn from(value: Keymap) -> Self {
        value.as_string()
    }
}

impl TryFrom<String> for Keymap {
    type Error = KeymapParseError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(&value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeymapParseError {
    InvalidKeyCount { expected: usize, found: usize },
}

impl fmt::Display for KeymapParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidKeyCount { expected, found } => {
                write!(
                    f,
                    "keymap must contain exactly {expected} keys, got {found}"
                )
            }
        }
    }
}

impl std::error::Error for KeymapParseError {}
