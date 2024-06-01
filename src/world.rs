
use petgraph::{
    graph::{NodeIndex, UnGraph},
    visit::EdgeRef,
};
use serde::{Deserialize, Serialize};

type PlaceKey = usize;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Location(PlaceKey, NodeIndex);

/// A physical place in the world, a dungeon, town, etc
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Place {
    name: String,
    place_key: PlaceKey,
    place_type: PlaceType,
    description: String,
    entrance: NodeIndex,
    rooms: UnGraph<Room, ()>,
}

impl Place {
    pub fn new(
        nd: (String, String),
        place_type: PlaceType,
        entrance: NodeIndex,
        rooms: UnGraph<Room, ()>,
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

    pub fn place_type(&self) -> PlaceType {
        self.place_type
    }

    pub(super) fn store(mut self, place_key: PlaceKey) -> Self {
        self.place_key = place_key;

        self
    }

    pub fn name(&self) -> &str {
        &self.name
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
