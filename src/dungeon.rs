use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::{core::{Attributes, Conversation}, web_types::Keyed};

#[derive(Debug, Deserialize, Serialize, Default)]
#[serde(default)]
pub struct Dungeon {
    #[serde(rename = "dungeon_name")]
    pub name: String,
    pub size: DungeonSize,
    pub difficulty: DungeonLevel,
    pub rooms: Vec<Room>,
    pub visited: bool,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Room {
    pub name: String,
    pub description: String,
    pub danger_level: DangerLevel,
    pub enemies: Vec<String>,
    #[serde(default)]
    pub connections: HashMap<Direction, String>,
}

impl Keyed for Dungeon {
    fn get_key() -> &'static str {
        "dungeons"
    }

    fn name(&self) -> &str {
        &self.name
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum DangerLevel {
    Safe,
    Caution,
    Danger,
}

impl Default for DangerLevel {
    fn default() -> Self {
        DangerLevel::Safe
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum DungeonSize {
    Small,
    Medium,
    Large,
    Huge,
}

#[derive(Debug, Deserialize, Serialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum DungeonLevel {
    Low,
    Medium,
    High,
}

impl DungeonSize {
    fn room_number(&self) -> &str {
        match self {
            DungeonSize::Small => "five",
            DungeonSize::Medium => "ten",
            DungeonSize::Large => "fourteen",
            DungeonSize::Huge => "twenty",
        }
    }
}

impl DungeonLevel {
    fn character_level(&self) -> &str {
        match self {
            DungeonLevel::Low => "low level",
            DungeonLevel::Medium => "mid level",
            DungeonLevel::High => "high level",
        }
    }
}

impl Default for DungeonSize {
    fn default() -> Self {
        DungeonSize::Medium
    }
}

impl Default for DungeonLevel {
    fn default() -> Self {
        DungeonLevel::Medium
    }
}

impl Dungeon {
    pub fn room(&self, name: &str) -> Option<&Room> {
        self.rooms.iter().find(|r| r.name == name)
    }
}

#[derive(Debug, Deserialize, Serialize, Hash, PartialEq, Eq, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    North,
    East,
    South,
    West,
    Up,
    Down,
}

impl Default for Direction {
    fn default() -> Self {
        Direction::North
    }
}

impl Direction {
    pub fn from_str(name: &str) -> Direction {
        match name {
            "North" => Direction::North,
            "East" => Direction::East,
            "South" => Direction::South,
            "West" => Direction::West,
            "Up" => Direction::Up,
            "Down" => Direction::Down,
            _ => Direction::North,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct Connector {
    name: String,
    exits: HashMap<Direction, String>,
}

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

pub struct DungeonGenerator(pub String, pub DungeonLevel, pub DungeonSize);

impl DungeonGenerator {
    pub async fn generate(self) -> anyhow::Result<(Dungeon, HashSet<String>)> {
        let name = self.0;
        let difficulty = self.1;
        let size = self.2;

        let prompt = include_str!("../primers/rooms.yaml")
            .replace("{size}", size.room_number())
            .replace("{level}", difficulty.character_level());
        let mut con = crate::core::Conversation::prime(&prompt);
        let json = con.say(&name).await?.1;

        let mut result = serde_json::from_str::<Vec<Room>>(&json);
        if result.is_err() {
            result = serde_json::from_str::<Dungeon>(&json).map(|d| d.rooms);
        }

        let rooms: Vec<Room> = result?;
        let enemy_types = rooms.iter().flat_map(|r| &r.enemies).cloned().collect();

        let dungeon = Dungeon {
            name: name.to_owned(),
            size,
            difficulty,
            rooms,
            visited: false,
        };

        Ok((dungeon, enemy_types))
    }
}

#[tokio::test]
async fn test_dungeon_generation() {
    dotenvy::dotenv().expect(".env file not found");
    let dungeon = DungeonGenerator("Shadowspire".to_owned(), DungeonLevel::Low, DungeonSize::Medium).generate().await.unwrap();
    println!("{:#?}", dungeon);
    assert_eq!(dungeon.0.rooms.len(), 10);
}

#[tokio::test]
async fn test_dungeon_connection() {
    dotenvy::dotenv().expect(".env file not found");
    let mut con = Conversation::prime(include_str!("../primers/connector.yaml"));
    let connected = con.query(r#"["Entrance Hall", "Crumbled Library", "Spider's Den", "Mossy Cellar", "Torture Chamber", "Collapsed Passageway", "Altar of Darkness", "Treasure Vault", "Chasm Bridge", "Throne Room"]"#).await.unwrap();
    let connections: Vec<Connector> = serde_json::from_str(&connected.1).unwrap();
    println!("{:#?}", connections);
}