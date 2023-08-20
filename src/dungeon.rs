use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::{
    core::{Attributes, Conversation},
    web_types::Keyed,
};

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
    pub traps: bool,
    pub secret_passage: bool,
    pub treasure: bool,
    #[serde(default)]
    pub connections: HashMap<Direction, String>,
}

impl Room {
    fn make_outside() -> Self {
        Self {
            name: "Outside".to_owned(),
            description: "A safe space just outside the dungeon".to_owned(),
            danger_level: DangerLevel::Safe,
            enemies: Vec::new(),
            traps: false,
            secret_passage: false,
            treasure: false,
            connections: HashMap::new(),
        }
    }
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
        let mut rooms_con = Conversation::prime(&prompt);
        let mut connector_con = Conversation::prime(include_str!("../primers/connector.yaml"));
        let json = rooms_con.say(&name).await?.1;

        let mut result = serde_json::from_str::<Vec<Room>>(&json);
        if result.is_err() {
            result = serde_json::from_str::<Dungeon>(&json).map(|d| d.rooms);
        }

        let mut rooms: Vec<Room> = result?;
        rooms.insert(0, Room::make_outside());

        let room_names: Vec<String> = rooms.iter().map(|r| &r.name).cloned().collect();
        let room_name_set: HashSet<&String> = rooms.iter().map(|r| &r.name).collect();

        if room_names.len() != room_name_set.len() {
            anyhow::bail!("Dungeon contained duplicate room names");
        }

        let connector_json = connector_con
            .say(&serde_json::to_string(&room_names)?)
            .await?
            .1;
        let connections: HashMap<String, HashMap<Direction, String>> =
            serde_json::from_str(&connector_json)?;

        for room in &mut rooms {
            if let Some(c) = connections.get(&room.name) {
                if c.values().all(|r| room_names.contains(&r)) {
                    room.connections = c.clone();
                } else {
                    anyhow::bail!("Room in connection map did not exist");
                }
            }
        }

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
    let dungeon = DungeonGenerator(
        "Shadowspire Dungeon".to_owned(),
        DungeonLevel::Low,
        DungeonSize::Huge,
    )
    .generate()
    .await
    .unwrap();
    println!("{:#?}", dungeon);
    assert_eq!(dungeon.0.rooms.len(), 21);
}
