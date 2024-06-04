use core::fmt;
use std::{
    collections::{HashMap, HashSet},
    fmt::{Debug, Display},
};

use serde::{Deserialize, Serialize};

use crate::{state::{self, PlayerId}, AppErrors};

use super::character::Character;

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct World {
    pub places: HashMap<Location, Place>,
    pub overworld_locales: Vec<Location>,
    pub player_characters: HashMap<PlayerId, Character>,
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
    pub fn tick_and_check_save(&mut self, interval: u64) {
        self.current_tick += 1;

        if self.current_tick % interval == 0 {
            let world_copy = self.clone();
            // We don't have an async context to use for IO here so save on a seperate thread
            std::thread::spawn(move || {
                let yaml = serde_yaml::to_string(&world_copy);
                let save = match yaml {
                    Ok(y) => std::fs::write(state::make_save_path("world.yaml"), y),
                    Err(e) => Ok(tracing::error!("Failed serializing world: {e}")),
                };

                if let Err(e) = save {
                    tracing::error!("Failed saving world: {e}");
                }
            });
        }
    }
}

/// A physical place in the world, a dungeon, town hall, etc.
/// One contiguous space, could be a busy market square or a holy temple's inner sanctum.
/// If it makes sense to draw battle lines along it's borders, you're on the right track.
/// Also used for the overland map.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Place {
    pub name: String,
    pub location: Location,
    pub description: String,
    pub tags: HashSet<String>,
    connections: HashMap<Direction, Location>,
}

impl Place {
    pub fn new(name: String, description: String) -> Self {
        Self {
            name,
            description,
            location: Location::new_location(),
            tags: Default::default(),
            connections: Default::default(),
        }
    }

    /// Add a new connection from this place to the provided location.
    /// Tries the provided direction, picks the first free one if that one is taken.
    /// Returns the direction used so you can try sync up the other side.
    pub fn add_connection(
        &mut self,
        direction: Direction,
        location: Location,
    ) -> Result<Direction, AppErrors> {
        let directions = Direction::values();
        if self.connections.len() >= directions.len() {
            return Err(AppErrors::TooManyConnections(self.location));
        }

        if self.connections.contains_key(&direction) {
            let next_dir = directions
                .iter()
                .filter(|d| !self.connections.contains_key(&d))
                .next()
                .unwrap();

            self.connections.insert(*next_dir, location);
            Ok(*next_dir)
        } else {
            self.connections.insert(direction, location);
            Ok(direction)
        }
    }

    /// Checks if a given location is directly adjacent to this one
    pub fn is_connected(&self, location: Location) -> bool {
        for (_, l) in &self.connections {
            if *l == location {
                return true;
            }
        }
        false
    }

    pub fn connections(&self) -> &HashMap<Direction, Location> {
        &self.connections
    }

    /// Generates the "look" text for the given place, describing what your character can see
    pub fn look(&self, world: &World, start: &str) -> String {
        let mut look_msg = format!("{start} {}\n\n{}\n\n", self.name, self.description);
        for (dir, loc) in self.connections() {
            look_msg.push_str(&format!(
                "Looking {} you see {}\n",
                dir.name(),
                world.places[loc].name
            ));
        }
        look_msg
    }
}

/// A unique key for each Place. It's default state is invalid,
/// to get a valid new Location call `new_location`.
#[derive(Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq, Hash)]
#[serde(transparent)]
pub struct Location(
    #[serde(
        serialize_with = "state::serialize_u128_hex",
        deserialize_with = "state::deserialize_u128_hex"
    )]
    u128,
);

impl Location {
    /// Generate a new location, ensuring we'll never get 0 which we
    /// consider invalid.
    fn new_location() -> Self {
        let v: u128 = rand::random();
        Self(v.saturating_add(1))
    }
}

impl Display for Location {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let location = self.0;
        f.write_fmt(format_args!("Location ({location:x})"))
    }
}

impl Debug for Location {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(&self, f)
    }
}

/// Represents the directions that players can move
#[derive(Debug, Serialize, Deserialize, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "kebab-case")]
pub enum Direction {
    North,
    East,
    South,
    West,
    Up,
    Down,
}

impl Direction {
    pub fn values() -> [Self; 6] {
        [
            Direction::North,
            Direction::East,
            Direction::South,
            Direction::West,
            Direction::Up,
            Direction::Down,
        ]
    }

    pub fn reverse(self) -> Self {
        match self {
            Direction::North => Direction::South,
            Direction::East => Direction::West,
            Direction::South => Direction::North,
            Direction::West => Direction::East,
            Direction::Up => Direction::Down,
            Direction::Down => Direction::Up,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Direction::North => "north",
            Direction::East => "east",
            Direction::South => "south",
            Direction::West => "west",
            Direction::Up => "up",
            Direction::Down => "down",
        }
    }
}
