use serde::{Deserialize, Serialize};

use crate::{
    core::Attributes,
    core::{Attribute, Location},
};

#[derive(Debug, Serialize, Deserialize)]
pub struct Combat {
    combatants: Vec<FighterRef>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum FighterRef {
    Player(String),
    Creature(Location, usize),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Action {
    Attack(FighterRef, Attribute),
    Evade,
    Push,
    Escape,
}

pub trait Fighter {
    fn damage(&mut self, damage: u32);
    fn health(&self) -> u32;
    fn attributes(&self) -> &Attributes;
}

impl Combat {
    pub fn new() -> Self {
        Self {
            combatants: Vec::new(),
        }
    }

    pub fn take_turn(&mut self) {}
}

impl FighterRef {}
