pub mod conversation;
pub mod schema;
pub mod dungeon;

use std::collections::HashMap;

use dotenvy::dotenv;

use crate::{conversation::Conversation, dungeon::Dungeon};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().expect(".env file not found");

    let mut c = Conversation::prime(include_str!("../primers/rooms.yaml"));
    let result = c.query("dungeon_name: Hell").await?.1;
    println!("{result}");
    let dungeon = Dungeon::from_schema(&serde_yaml::from_str::<schema::DungeonSchema>(&result)?, &mut HashMap::new());
    println!("{dungeon:#?}");

    Ok(())
}
