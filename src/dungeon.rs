use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::schema::{Attributes, Direction, DungeonSchema, EnemyPlacement, RoomSchema};

#[derive(Debug, Deserialize, Serialize)]
pub struct Dungeon {
    name: String,
    rooms: Vec<Room>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Room {
    name: String,
    description: String,
    enemies: Vec<Enemy>,
    connections: HashMap<Direction, String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Enemy {
    name: String,
    attributes: Attributes,
}

impl Dungeon {
    pub fn from_schema(
        schema: &DungeonSchema,
        enemy_registry: &mut HashMap<String, Enemy>,
    ) -> Self {
        // TODO: Use this to pre-populate the enemies registry
        let enemies: Vec<EnemyPlacement> = schema
            .rooms
            .iter()
            .flat_map(|r| r.enemies.clone())
            .collect();
        let rooms = schema
            .rooms
            .iter()
            .map(|r| {
                (
                    r.clone(),
                    link_connections(r, &schema.rooms),
                    create_enemies(r, &enemy_registry),
                )
            })
            .map(Room::from_schema)
            .collect();

        Self {
            name: schema.name.to_owned(),
            rooms,
        }
    }
}

impl Room {
    pub fn from_schema(schema: (RoomSchema, HashMap<Direction, String>, Vec<Enemy>)) -> Self {
        let (schema, connections, enemies) = schema;
        Self {
            name: schema.name.to_owned(),
            description: schema.description.to_owned(),
            enemies,
            connections,
        }
    }
}

/// Turn the uni-directional graph we get from the AI
/// into a bi-directional graph
fn link_connections(schema: &RoomSchema, rooms: &Vec<RoomSchema>) -> HashMap<Direction, String> {
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

/// Creates enemies by referencing the existing registry
fn create_enemies(schema: &RoomSchema, enemy_registry: &HashMap<String, Enemy>) -> Vec<Enemy> {
    // TODO: Use enemy registry and counts
    schema
        .enemies
        .iter()
        .map(|ep| Enemy {
            name: ep.name.to_owned(),
            attributes: Attributes::default(),
        })
        .collect()
}
