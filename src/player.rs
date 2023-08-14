use serde::{Deserialize, Serialize};

use crate::core::{Attributes, Location};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Player {
    pub name: String,
    pub salt: String,
    pub password: [u8; 32],
    pub location: Location,
    attributes: Attributes,
}

impl Player {
    pub fn new(name: String, password: [u8; 32], salt: String) -> Self {
        Self {
            name,
            password,
            salt,
            location: Location::default(),
            attributes: Attributes::default(),
        }
    }

    pub fn creation_complete(&self) -> bool {
        !self.location.is_empty()
    }
}
