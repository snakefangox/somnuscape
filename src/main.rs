pub mod conversation;

use dotenvy::dotenv;
use serde::{Serialize, Deserialize};

use crate::conversation::Conversation;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().expect(".env file not found");

    let mut c = Conversation::new("gpt-3.5-turbo", "You are an expert tabletop rpg dungeon designer, you produce dungeon layouts in yaml");
    c.add_message("dungeon_name: The Frozen Caverns");
    c.add_assistant_message(include_str!("../examples/rooms.yaml"));
    c.add_message("dungeon_name: The Shadow Maze");

    println!("{:?}", c.send().await?.1);

    Ok(())
}


#[derive(Debug, Serialize, Deserialize)]
pub struct Room {
    name: String,
    description: String,
}
