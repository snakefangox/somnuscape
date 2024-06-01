use anyhow::Result;
use askama::Template;
use serde::{Deserialize, Serialize};

use crate::{generation, mud::character::Attributes};

use super::AIClient;

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
    pub async fn stat_new(client: &AIClient, creature_name: &str) -> Result<Self> {
        let res = client
            .generate(
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
    async fn generate_sensible_creatures() {
        if std::env::var("SKIP_AI_TESTS").unwrap_or_default() == "yes" {
            return;
        }
        let client = AIClient::default();

        let minotaur = CreatureTemplate::stat_new(&client, "minotaur").await.unwrap();
        assert!(minotaur.attributes.strength > minotaur.attributes.intelligence);
        assert!(minotaur.attributes.toughness > minotaur.attributes.willpower);

        let lich = CreatureTemplate::stat_new(&client, "lich").await.unwrap();
        assert!(lich.attributes.intelligence > lich.attributes.strength);
        assert!(lich.attributes.willpower > lich.attributes.toughness);
    }
}
