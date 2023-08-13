use std::fmt::Display;

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

#[derive(Debug, Serialize, Deserialize, Default)]
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

pub type Message = (Role, String);
const CHAT_GPT: &str = "gpt-3.5-turbo";

#[derive(Debug, Clone)]
pub struct Conversation {
    messages: Vec<Message>,
    client: Client<OpenAIConfig>,
}

impl Conversation {
    pub fn new(system_prompt: &str) -> Self {
        Self {
            client: Client::new(),
            messages: vec![(Role::System, system_prompt.to_string())],
        }
    }

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
        }
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
    pub async fn request(&mut self, msg: &str) -> Result<Message, OpenAIError> {
        self.add_message(msg);
        let answer = self.send().await?;
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
