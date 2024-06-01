use petgraph::{graph::NodeIndex, stable_graph::StableUnGraph, visit::EdgeRef};
use serde::{Deserialize, Serialize};

use crate::state;

type PlaceKey = usize;

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct World {
    pub places: Vec<Place>,
    pub current_tick: u64,
}

impl World {
    pub fn load_or_default() -> Self {
        let p = state::make_save_path("world.yaml");
        if p.try_exists().unwrap_or_default() {
            std::fs::read_to_string(p)
                .and_then(|y| Ok(serde_yaml::from_str(&y)))
                .expect("Could not read save file")
                .expect("Could not deserialize")
        } else {
            Self::default()
        }
    }

    /// Increment the current tick count and then check and save if needed
    pub fn check_save(&mut self, interval: u64) {
        self.current_tick += 1;

        if self.current_tick % interval == 0 {
            let world_copy = self.clone();
            std::thread::spawn(move || {
                let yaml = serde_yaml::to_string(&world_copy);
                let save = match yaml {
                    Ok(y) => std::fs::write(state::make_save_path("world.yaml"), y),
                    Err(e) => Ok(tracing::error!("Error serializing world: {e}")),
                };

                if let Err(e) = save {
                    tracing::error!("Error saving world: {e}");
                }
            });
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Location(PlaceKey, NodeIndex);

/// A physical place in the world, a dungeon, town, etc
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Place {
    pub name: String,
    pub place_key: PlaceKey,
    pub place_type: PlaceType,
    description: String,
    entrance: NodeIndex,
    rooms: StableUnGraph<Room, ()>,
}

impl Place {
    pub fn new(
        nd: (String, String),
        place_type: PlaceType,
        entrance: NodeIndex,
        rooms: StableUnGraph<Room, ()>,
    ) -> Self {
        Self {
            name: nd.0,
            description: nd.1,
            place_key: 0,
            place_type,
            entrance,
            rooms,
        }
    }

    pub fn entrance(&self) -> Location {
        Location(self.place_key, self.entrance)
    }

    pub fn get_room<R: RoomIdx>(&self, idx: R) -> Option<&Room> {
        self.rooms.node_weight(idx.get_room_idx())
    }

    pub fn get_adj_rooms<'a, R: RoomIdx>(&'a self, idx: R) -> impl Iterator<Item = Location> + 'a {
        self.rooms
            .edges_directed(idx.get_room_idx(), petgraph::Direction::Outgoing)
            .map(|e| Location(self.place_key, e.target()))
    }
}

pub trait RoomIdx {
    fn get_room_idx(&self) -> NodeIndex;
}

impl RoomIdx for NodeIndex {
    fn get_room_idx(&self) -> NodeIndex {
        *self
    }
}

impl RoomIdx for Location {
    fn get_room_idx(&self) -> NodeIndex {
        self.1
    }
}

/// A contiguous space inside a place, could be a busy market square or a holy temple's inner sanctum
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Room {
    name: String,
    description: String,
}

impl Room {
    pub fn new(name: String, description: String) -> Self {
        Self { name, description }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn description(&self) -> &str {
        &self.description
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq)]
pub enum PlaceType {
    Village,
    Dungeon,
}

impl PlaceType {
    pub fn name(self) -> &'static str {
        match self {
            PlaceType::Village => "village",
            PlaceType::Dungeon => "dungeon",
        }
    }

    pub fn room_type(self) -> &'static str {
        match self {
            PlaceType::Village => "building, street or square",
            PlaceType::Dungeon => "room or corridor",
        }
    }
}
