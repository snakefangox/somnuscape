use serde::{Deserialize, Serialize};

use crate::core::{Attributes, Location};

#[derive(Debug, Serialize, Deserialize)]
pub struct Player {
    pub name: String,
    pub salt: String,
    pub password: [u8; 32],
    location: Location,
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
}
