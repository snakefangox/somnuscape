use serde::{Serialize, Deserialize};

use crate::core::{Attributes, Location};

#[derive(Debug, Serialize, Deserialize)]
pub struct Player {
    name: String,
    location: Location,
    attributes: Attributes,
}
