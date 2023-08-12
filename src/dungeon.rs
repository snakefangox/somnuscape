use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::creatures::Bestiary;

#[derive(Debug, Deserialize, Serialize)]
pub struct Dungeon {
    #[serde(rename = "dungeon_name")]
    pub name: String,
    pub rooms: Vec<Room>,
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
    pub async fn from_json(json: &str, enemy_registry: &mut Bestiary) -> Self {
        let mut dungeon: Dungeon = serde_json::from_str(json).unwrap();
        let enemy_types: HashSet<&String> = dungeon.rooms.iter().flat_map(|r| &r.enemies).collect();

        for enemy in enemy_types {
            enemy_registry.get_creature(&enemy).await;
        }

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

        dungeon
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
