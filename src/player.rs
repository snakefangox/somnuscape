use serde::{Deserialize, Serialize};

use crate::core::{AttributeRating, Attributes, Location};

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
        !self.location.is_character_creation()
    }

    pub fn finish_character_creation(
        &mut self,
        strength: AttributeRating,
        agility: AttributeRating,
        intelligence: AttributeRating,
    ) {
        self.attributes.strength = strength;
        self.attributes.agility = agility;
        self.attributes.intelligence = intelligence;
        self.location = Location::Town;
    }
}
