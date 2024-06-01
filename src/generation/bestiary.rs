use askama::Template;
use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::{characters::Attributes, generation};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatureTemplate {
    name: String,
    attributes: Attributes,
}

#[derive(Template, Default)]
#[template(path = "stat_creature.md")]
struct StatCreatureTemplate<'a> {
    creature_name: &'a str,
    attributes: &'a [&'a str],
}

impl CreatureTemplate {
    pub async fn stat_new(
        creature_name: &str,
        attributes: &[&str],
    ) -> Result<Self> {
        let res = generation::generate(
            StatCreatureTemplate {
                creature_name,
                attributes,
            }
            .to_string(),
        )
        .await?;

        Ok(generation::extract_yaml(&res)?)
    }
}
