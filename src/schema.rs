use std::collections::HashMap;

use serde::{Deserialize, Serialize};


#[derive(Debug, Deserialize, Serialize)]
pub struct CreatureSchema {
    #[serde(rename = "creature_name")]
    pub name: String,
    pub attributes: Attributes,
}

#[derive(Debug, Default, Deserialize, Serialize, Clone)]
pub struct Attributes {
    pub health: u8,
    pub strength: u8,
    pub agility: u8,
    pub intelligence: u8,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DungeonSchema {
    #[serde(rename = "dungeon_name")]
    pub name: String,
    pub rooms: Vec<RoomSchema>,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
#[serde(default)]
pub struct RoomSchema {
    pub name: String,
    pub description: String,
    pub enemies: Vec<EnemyPlacement>,
    pub connections: HashMap<Direction, String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct EnemyPlacement {
    pub name: String,
    pub count: u64,
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
    pub fn inverse(&self) -> Direction {
        match self {
            Direction::North => Self::South,
            Direction::East => Self::West,
            Direction::South => Self::North,
            Direction::West => Self::East,
            Direction::Up => Self::Down,
            Direction::Down => Self::Up,
        }
    }
}