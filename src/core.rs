use std::fmt::Display;

use serde::{Deserialize, Serialize};


#[derive(Debug, Serialize, Deserialize)]
pub struct Location {
    area: String,
    room: String,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum AttributeRating {
    Pathetic,
    Pitiful,
    Mediocre,
    Average,
    Decent,
    Good,
    Great,
    Excellent,
    Superb,
    Godly,
}

impl AttributeRating {
    pub fn rank(&self) -> u32 {
        match self {
            AttributeRating::Pathetic => 1,
            AttributeRating::Pitiful => 2,
            AttributeRating::Mediocre => 3,
            AttributeRating::Average => 4,
            AttributeRating::Decent => 5,
            AttributeRating::Good => 6,
            AttributeRating::Great => 7,
            AttributeRating::Excellent => 8,
            AttributeRating::Superb => 9,
            AttributeRating::Godly => 10,
        }
    }
}

impl Display for AttributeRating {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AttributeRating::Pathetic => f.write_str("Pathetic")?,
            AttributeRating::Pitiful => f.write_str("Pitiful")?,
            AttributeRating::Mediocre => f.write_str("Mediocre")?,
            AttributeRating::Average => f.write_str("Average")?,
            AttributeRating::Decent => f.write_str("Decent")?,
            AttributeRating::Good => f.write_str("Good")?,
            AttributeRating::Great => f.write_str("Great")?,
            AttributeRating::Excellent => f.write_str("Excellent")?,
            AttributeRating::Superb => f.write_str("Superb")?,
            AttributeRating::Godly => f.write_str("Godly")?,
        }

        if f.alternate() {
            f.write_fmt(format_args!(" ({})", self.rank()))?;
        }

        Ok(())
    }
}

impl Default for AttributeRating {
    fn default() -> Self {
        AttributeRating::Average
    }
}