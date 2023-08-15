use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::{
    creatures::{Bestiary, Creature},
    web_types::Keyed,
};

#[derive(Debug, Deserialize, Serialize)]
pub struct Dungeon {
    #[serde(rename = "dungeon_name")]
    pub name: String,
    pub rooms: Vec<Room>,
    #[serde(default)]
    pub visited: bool,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Room {
    pub name: String,
    pub description: String,
    pub enemies: Vec<String>,
    pub connections: HashMap<Direction, String>,
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

impl Dungeon {
    pub fn from_json(json: &str) -> anyhow::Result<(Self, HashSet<String>)> {
        let mut dungeon: Dungeon = serde_json::from_str(json)?;
        let enemy_types = dungeon
            .rooms
            .iter()
            .flat_map(|r| &r.enemies)
            .cloned()
            .collect();

        // TODO: Gotta be a better way than this...
        let rooms: Vec<HashMap<Direction, String>> = dungeon
            .rooms
            .iter()
            .map(|r| link_connections(&r, &dungeon.rooms))
            .collect();

        dungeon
            .rooms
            .iter_mut()
            .zip(rooms)
            .for_each(|(room, conn)| room.connections = conn);

        Ok((dungeon, enemy_types))
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

/// Turn the uni-directional graph we get from the AI into a bi-directional graph
fn link_connections(schema: &Room, rooms: &Vec<Room>) -> HashMap<Direction, String> {
    rooms
        .iter()
        .flat_map(|r| {
            r.connections
                .iter()
                .filter(|p| p.1 == &schema.name)
                .map(|d| (d.0.inverse(), r.name.to_owned()))
        })
        .chain(schema.connections.clone())
        .collect()
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
