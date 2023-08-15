use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::{core::{Attributes, Conversation}, web_types::Keyed};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Creature {
    #[serde(rename = "creature_name")]
    pub name: String,
    pub attributes: Attributes,
}

impl Keyed for Creature {
    fn get_key() -> &'static str {
        "creatures"
    }

    fn name(&self) -> &str {
        &self.name
    }
}

#[derive(Debug)]
pub struct Bestiary {
    stat_gen: Conversation,
    bestiary: HashMap<String, Creature>,
}

impl Bestiary {
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
            let result = serde_json::from_str::<Creature>(
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