use anyhow::Result;
use askama::Template;
use serde::{Deserialize, Serialize};

use crate::{generation, mud::character::Attributes};

use super::AIClient;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatureTemplate {
    pub name: String,
    pub attributes: Attributes,
    pub items: Vec<String>,
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
