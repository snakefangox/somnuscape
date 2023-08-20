use std::{collections::VecDeque, fmt::Display};

use async_openai::{
    config::OpenAIConfig,
    error::OpenAIError,
    types::{
        ChatCompletionRequestMessage, ChatCompletionRequestMessageArgs,
        CreateChatCompletionRequestArgs, Role,
    },
    Client,
};
use serde::{Deserialize, Serialize};

use crate::{player::Player, web_types::Keyed};

pub const STARTING_POINT_TOTAL: u32 = 12;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Location {
    CharacterCreation,
    Town,
    Dungeon { area: String, room: String },
}

impl Location {
    pub fn name(&self) -> String {
        match self {
            Location::CharacterCreation => "character_creation".to_owned(),
            Location::Town => "town".to_owned(),
            Location::Dungeon { area, room } => format!("{}:{}", area, room),
        }
    }

    pub fn area(&self) -> String {
        match self {
            Location::CharacterCreation | Location::Town => self.name(),
            Location::Dungeon { area, room: _ } => area.to_owned(),
        }
    }

    pub fn room(&self) -> String {
        match self {
            Location::CharacterCreation | Location::Town => self.name(),
            Location::Dungeon { area: _, room } => room.to_owned(),
        }
    }

    pub fn move_room(&mut self, new_room: &str) {
        match self {
            Location::CharacterCreation | Location::Town => (),
            Location::Dungeon { area: _, room } => *room = new_room.to_owned(),
        }
    }

    pub fn describe(&self) -> String {
        match self {
            Location::CharacterCreation => "You are a sea of possibilities, not yet complete".to_owned(),
            Location::Town => "You are safely back at town".to_owned(),
            Location::Dungeon { area, room } => format!("You are in {area}, standing in {room}"),
        }
    }

    pub fn is_character_creation(&self) -> bool {
        match self {
            Location::CharacterCreation => true,
            _ => false,
        }
    }

    pub fn is_town(&self) -> bool {
        match self {
            Location::Town => true,
            _ => false,
        }
    }

    pub fn is_dungeon(&self) -> bool {
        match self {
            Location::Dungeon { area: _, room: _ } => true,
            _ => false,
        }
    }
}

impl Default for Location {
    fn default() -> Self {
        Location::CharacterCreation
    }
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

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum Attribute {
    Strength,
    Agility,
    Intelligence,
}

#[derive(Debug, Default, Deserialize, Serialize, Clone)]
pub struct Attributes {
    pub health: AttributeRating,
    pub strength: AttributeRating,
    pub agility: AttributeRating,
    pub intelligence: AttributeRating,
}

impl AttributeRating {
    pub fn from_rank(rank: u32) -> Option<AttributeRating> {
        match rank {
            1 => Some(AttributeRating::Pathetic),
            2 => Some(AttributeRating::Pitiful),
            3 => Some(AttributeRating::Mediocre),
            4 => Some(AttributeRating::Average),
            5 => Some(AttributeRating::Decent),
            6 => Some(AttributeRating::Good),
            7 => Some(AttributeRating::Great),
            8 => Some(AttributeRating::Excellent),
            9 => Some(AttributeRating::Superb),
            10 => Some(AttributeRating::Godly),
            _ => None,
        }
    }

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

    pub fn max_health(&self) -> u32 {
        (self.rank() * 2) + 2
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

pub type Message = (Role, String);
const CHAT_GPT: &str = "gpt-3.5-turbo";

#[derive(Debug, Clone)]
pub struct Conversation {
    temprature: f32,
    messages: Vec<Message>,
    client: Client<OpenAIConfig>,
}

impl Conversation {
    /// Start a conversation with some primer text already loaded
    /// Accepts a string seperated by lines of --- into messages
    /// The first message is the system message, then it alternates: User, Assistant
    pub fn prime(primer: &str) -> Self {
        let msgs = primer
            .split("\n---\n")
            .enumerate()
            .map(|(i, m)| {
                let role = match i {
                    0 => Role::System,
                    i if i % 2 == 0 => Role::Assistant,
                    _ => Role::User,
                };
                (role, m.to_owned())
            })
            .collect();

        Self {
            client: Client::new(),
            messages: msgs,
            temprature: 0.,
        }
    }

    /// Set the model temprature
    pub fn temprature(&mut self, temp: f32) {
        self.temprature = temp;
    }

    /// Add a user message to the conversation
    pub fn add_message(&mut self, msg: &str) {
        self.messages.push((Role::User, msg.to_string()));
    }

    /// Temporarily adds a message to the conversation and appends and returns the response
    /// Useful for querying a primed conversation
    pub async fn query(&mut self, msg: &str) -> Result<Message, OpenAIError> {
        self.add_message(msg);
        let answer = self.send().await?;
        self.messages.pop();
        Ok(answer)
    }

    /// Adds a message to the conversation and appends and returns the response
    pub async fn say(&mut self, msg: &str) -> Result<Message, OpenAIError> {
        self.add_message(msg);
        let answer = self.send().await?;
        self.messages.push(answer.clone());
        Ok(answer)
    }

    /// Sends the conversation to the AI and returns the response
    pub async fn send(&self) -> Result<Message, OpenAIError> {
        let messages: Vec<ChatCompletionRequestMessage> = self
            .messages
            .iter()
            .map(|(r, m)| {
                ChatCompletionRequestMessageArgs::default()
                    .role(r.clone())
                    .content(m)
                    .build()
                    .unwrap()
            })
            .collect();

        let chat_req = CreateChatCompletionRequestArgs::default()
            .model(CHAT_GPT)
            .temperature(self.temprature)
            .messages(messages)
            .build()?;

        let answer = self.client.chat().create(chat_req).await?;
        let msg = &answer.choices[0].message;
        let msg = (
            msg.role.clone(),
            msg.content.clone().unwrap_or_default().clone(),
        );

        Ok(msg)
    }
}

const CHAT_HISTORY: usize = 30;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ChatContext {
    pub name: String,
    messages: VecDeque<(String, String)>,
}

impl ChatContext {
    pub fn new(location: &Location) -> Self {
        Self {
            name: location.name(),
            messages: VecDeque::default(),
        }
    }

    pub fn send_msg(&mut self, player: &Player, msg: String) {
        self.messages.push_back((player.name.to_owned(), msg));
        if self.messages.len() > CHAT_HISTORY {
            self.messages.pop_front();
        }
    }

    pub fn messages(&self) -> &VecDeque<(String, String)> {
        &self.messages
    }
}

impl Keyed for ChatContext {
    fn get_key() -> &'static str {
        "chats"
    }

    fn name(&self) -> &str {
        &self.name
    }
}
