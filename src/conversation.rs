use async_openai::{
    error::OpenAIError,
    types::{
        ChatCompletionRequestMessage, ChatCompletionRequestMessageArgs,
        CreateChatCompletionRequestArgs, Role,
    },
    Client, config::OpenAIConfig,
};

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
