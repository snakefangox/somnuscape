use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::conversation::Conversation;

#[derive(Debug)]
pub struct CreatureRegistry {
    stat_gen: Conversation,
    bestiary: HashMap<String, Creature>,
}

impl CreatureRegistry {
    pub fn new() -> Self {
        Self {
            stat_gen: Conversation::prime(include_str!("../primers/stats.yaml")),
            bestiary: HashMap::new(),
        }
    }

    /// Gets a creature schema from a name, either from the
    /// cache or from the AI
    pub async fn get_creature(&mut self, name: &str) -> Creature {
        if self.bestiary.contains_key(name) {
            self.bestiary[name].clone()
        } else {
            let result = serde_yaml::from_str::<Creature>(
                &self
                    .stat_gen
                    .query(&format!("creature_name: {}", name))
                    .await
                    .unwrap()
                    .1,
            )
            .unwrap();

            self.bestiary.insert(name.to_owned(), result.clone());
            result
        }
    }

    /// Gets a creature and panics if it isn't present
    pub fn get_creature_unwrap(&self, name: &str) -> Creature {
        self.bestiary[name].clone()
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Creature {
    #[serde(rename = "creature_name")]
    pub name: String,
    pub attributes: Attributes,
}

#[derive(Debug, Default, Deserialize, Serialize, Clone)]
pub struct Attributes {
    pub health: AttributeRating,
    pub strength: AttributeRating,
    pub agility: AttributeRating,
    pub intelligence: AttributeRating,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum AttributeRating {
    Pathetic,
    Pitiful,
    Mediocre,
    Average,
    Decent,
    Good,
    Great,
    Excellent,
    Superb,
    Godly,
}

impl AttributeRating {
    pub fn rank(&self) -> u32 {
        match self {
            AttributeRating::Pathetic => 1,
            AttributeRating::Pitiful => 2,
            AttributeRating::Mediocre => 3,
            AttributeRating::Average => 4,
            AttributeRating::Decent => 5,
            AttributeRating::Good => 6,
            AttributeRating::Great => 7,
            AttributeRating::Excellent => 8,
            AttributeRating::Superb => 9,
            AttributeRating::Godly => 10,
        }
    }
}

impl Default for AttributeRating {
    fn default() -> Self {
        AttributeRating::Average
    }
}