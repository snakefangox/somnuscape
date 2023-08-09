use async_openai::{
    types::{
        ChatCompletionRequestMessage, ChatCompletionRequestMessageArgs,
        CreateChatCompletionRequestArgs, Role,
    },
    Client, error::OpenAIError,
};

pub type Message = (Role, String);

#[derive(Debug)]
pub struct Conversation {
    model: String,
    messages: Vec<Message>,
}

impl Conversation {
    pub fn new(model: &str, system_prompt: &str) -> Self {
        Self {
            model: model.to_string(),
            messages: vec![(Role::System, system_prompt.to_string())],
        }
    }

    pub fn add_message(&mut self, msg: &str) {
        self.messages.push((Role::User, msg.to_string()));
    }

    pub fn add_assistant_message(&mut self, msg: &str) {
        self.messages.push((Role::Assistant, msg.to_string()));
    }

    pub async fn send(&mut self) -> Result<Message, OpenAIError> {
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
            .model(self.model.clone())
            .messages(messages)
            .build()?;

        let answer = Client::new().chat().create(chat_req).await?;
        let msg = &answer.choices[0].message;
        let msg = (
            msg.role.clone(),
            msg.content.clone().unwrap_or_default().clone(),
        );

        self.messages.push(msg.clone());

        Ok(msg)
    }
}
