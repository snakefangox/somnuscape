use serde::{Serialize, Deserialize};

use crate::{creatures::Attributes, core::Location};

#[derive(Debug, Serialize, Deserialize)]
pub struct Player {
    name: String,
    location: Location,
    attributes: Attributes,
}
