use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::{
    creatures::{CreatureRegistry, Creature},
    schema::{Direction, DungeonSchema, RoomSchema},
};

#[derive(Debug, Deserialize, Serialize)]
pub struct Dungeon {
    name: String,
    rooms: Vec<Room>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Room {
    name: String,
    description: String,
    enemies: Vec<Creature>,
    connections: HashMap<Direction, String>,
}

impl Dungeon {
    pub async fn from_schema(
        schema: &DungeonSchema,
        enemy_registry: &mut CreatureRegistry,
    ) -> Self {
        // TODO: Use this to pre-populate the enemies registry
        let enemy_types: HashSet<String> = schema
            .rooms
            .iter()
            .flat_map(|r| r.enemies.clone())
            .map(|ep| ep.name)
            .collect();

        for enemy in enemy_types {
            enemy_registry.get_creature(&enemy).await;
        }

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
    pub fn from_schema(schema: (RoomSchema, HashMap<Direction, String>, Vec<Creature>)) -> Self {
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
fn create_enemies(schema: &RoomSchema, enemy_registry: &CreatureRegistry) -> Vec<Creature> {
    let mut enemies = Vec::new();
    for ep in &schema.enemies {
        let enemy = enemy_registry.get_creature_unwrap(&ep.name);
        for _ in 0..ep.count {
            enemies.push(enemy.clone());
        }
    }

    enemies
}
