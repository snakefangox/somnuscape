pub mod conversation;
pub mod creatures;
pub mod dungeon;
pub mod schema;

use dotenvy::dotenv;
use rand::seq::SliceRandom;
use regex::Regex;

use crate::{conversation::Conversation, creatures::CreatureRegistry, dungeon::Dungeon};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().expect(".env file not found");

    let mut c = Conversation::prime(include_str!("../primers/names.yaml"));
    let names = c.query("10 fantasy dungeon names").await?.1;
    let r = Regex::new(r"(?m)^[0-9]+\. (.*)$")?;
    let mut names: Vec<String> = r
        .captures_iter(&names)
        .filter_map(|c| c.get(1).map(|c| c.as_str().to_owned()))
        .collect();
    println!("{names:#?}");
    names.shuffle(&mut rand::thread_rng());
    let name = names.pop().unwrap();

    let mut creature_registry = CreatureRegistry::new();
    let mut c = Conversation::prime(include_str!("../primers/rooms.yaml"));
    let result = c.request(&format!("dungeon_name: {name}")).await?.1.replace('\'', "\\'");
    println!("{result}");
    let dungeon = Dungeon::from_schema(&serde_yaml::from_str::<schema::DungeonSchema>(&result)?, &mut creature_registry).await;
    println!("{dungeon:#?}");

    Ok(())
}
