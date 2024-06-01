use anyhow::Result;
use askama::Template;
use serde::{Deserialize, Serialize};

use crate::{characters::Attributes, generation};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatureTemplate {
    name: String,
    attributes: Attributes,
    items: Vec<String>,
}

#[derive(Template, Default)]
#[template(path = "stat_creature.md")]
struct StatCreatureTemplate<'a> {
    creature_name: &'a str,
    attributes: &'a [&'a str],
}

impl CreatureTemplate {
    pub async fn stat_new(creature_name: &str) -> Result<Self> {
        let res = generation::generate(
            StatCreatureTemplate {
                creature_name,
                attributes: &[
                    "strength",
                    "toughness",
                    "agility",
                    "intelligence",
                    "willpower",
                ],
            }
            .to_string(),
        )
        .await?;

        Ok(generation::extract_yaml(&res)?)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    async fn generate_valid_creatures() {
        let minotaur = CreatureTemplate::stat_new("minotaur").await.unwrap();
        println!("{minotaur:?}")
    }
}
