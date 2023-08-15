use serde::{Deserialize, Serialize};

use crate::{core::{AttributeRating, Attributes, Location}, web_types::Keyed};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Player {
    pub name: String,
    pub location: Location,
    pub health: u32,
    pub attributes: Attributes,
}
 
impl Player {
    pub fn new(name: String) -> Self {
        Self {
            name,
            location: Location::default(),
            health: AttributeRating::default().max_health(),
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

impl Keyed for Player {
    fn get_key() -> &'static str {
        "players"
    }

    fn name(&self) -> &str {
        &self.name
    }
}