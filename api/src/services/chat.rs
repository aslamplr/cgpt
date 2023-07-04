use std::{borrow::Cow, ops::Deref};

use async_openai::types::{ChatCompletionRequestMessage, ChatCompletionRequestMessageArgs, Role};
use serde::{Deserialize, Serialize};
use surrealdb::{engine::remote::ws::Client as Ws, sql::Thing, Surreal};

use crate::openai::Client;
use crate::util::generate_chat_id;

use super::AppState;

#[derive(Deserialize)]
pub struct ChatRequest {
    pub message: Cow<'static, str>,
}

#[derive(Serialize)]
pub struct ChatResponse {
    chat_id: Cow<'static, str>,
    message: Cow<'static, str>,
}

#[derive(Serialize)]
pub struct ChatHistory {
    chat_id: Cow<'static, str>,
    messages: Vec<Cow<'static, str>>,
}

#[derive(Serialize)]
pub struct ChatList {
    chats: Vec<Cow<'static, str>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Chat {
    chat_id: Cow<'static, str>,
    messages: Vec<ChatMessage>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatMessage(ChatCompletionRequestMessage);

#[derive(Debug, Deserialize)]
pub struct Record {
    #[allow(dead_code)]
    id: Thing,
}

impl Deref for ChatMessage {
    type Target = ChatCompletionRequestMessage;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<ChatCompletionRequestMessage> for ChatMessage {
    fn from(message: ChatCompletionRequestMessage) -> Self {
        Self(message)
    }
}

impl<T: Into<Cow<'static, str>>, K: Into<ChatMessage>> From<(T, Vec<K>)> for Chat {
    fn from((chat_id, messages): (T, Vec<K>)) -> Self {
        Self {
            chat_id: chat_id.into(),
            messages: messages.into_iter().map(Into::into).collect(),
        }
    }
}

impl AppState {
    pub fn new(openai_client: Client, db: Surreal<Ws>) -> Self {
        Self { openai_client, db }
    }

    pub async fn new_chat(&self, message: &str) -> anyhow::Result<ChatResponse> {
        let chat_id = generate_chat_id();
        let mut messages = vec![ChatCompletionRequestMessageArgs::default()
            .role(Role::System)
            .content(
                "You are a helpful software engineer expert in Rust language and AWS Cloud Platform.",
            )
            .build()?,
            ChatCompletionRequestMessageArgs::default()
                .role(Role::User)
                .content(message)
                .build()?
        ];

        let response = self.openai_client.chat(&messages).await?;

        let response = response.choices.first().cloned();
        if let Some(response) = response {
            let content = response
                .message
                .content
                .unwrap_or_else(|| "No content in response!".to_string());
            messages.push(
                ChatCompletionRequestMessageArgs::default()
                    .role(response.message.role)
                    .content(content.clone())
                    .build()?,
            );
            let chat = Chat::from((chat_id.clone(), messages));
            let updated: Record = self.db.create(("chat", chat_id)).content(chat).await?;
            let chat_id = updated
                .id
                .to_string()
                .split(':')
                .last()
                .unwrap_or_default()
                .to_string();
            Ok(ChatResponse {
                chat_id: chat_id.into(),
                message: content.into(),
            })
        } else {
            Ok(ChatResponse {
                chat_id: "none".into(),
                message: "No response!".into(),
            })
        }
    }

    pub async fn list_chat(&self) -> anyhow::Result<ChatList> {
        let chats: Vec<Chat> = self.db.select("chat").await?;
        let chats = chats
            .into_iter()
            .map(|c| c.chat_id)
            .collect::<Vec<Cow<'static, str>>>();
        Ok(ChatList { chats })
    }

    pub async fn get_chat(&self, chat_id: &str) -> anyhow::Result<ChatHistory> {
        let chat: Option<Chat> = self.db.select(("chat", chat_id)).await?;
        if let Some(chat) = chat {
            let messages = chat.messages;
            let messages = messages
                .into_iter()
                .map(|c| c.content.clone().unwrap_or_default().into())
                .collect::<Vec<Cow<'static, str>>>();
            Ok(ChatHistory {
                chat_id: chat_id.to_string().into(),
                messages,
            })
        } else {
            Ok(ChatHistory {
                chat_id: "none".into(),
                messages: vec![],
            })
        }
    }

    pub async fn continue_chat(
        &self,
        chat_id: &str,
        message: &str,
    ) -> anyhow::Result<ChatResponse> {
        let chat: Option<Chat> = self.db.select(("chat", chat_id)).await?;
        if let Some(chat) = chat {
            let mut messages = chat
                .messages
                .into_iter()
                .map(|c| ChatCompletionRequestMessage {
                    role: c.role.clone(),
                    content: c.content.clone(),
                    name: None,
                    function_call: None,
                })
                .collect::<Vec<ChatCompletionRequestMessage>>();
            let args = ChatCompletionRequestMessageArgs::default()
                .role(Role::User)
                .content(message)
                .build()?;
            messages.push(args);

            let response = self.openai_client.chat(&messages).await?;

            let response = response.choices.first().cloned();
            if let Some(response) = response {
                let content = response
                    .message
                    .content
                    .unwrap_or_else(|| "No content in response!".to_string());
                messages.push(
                    ChatCompletionRequestMessageArgs::default()
                        .role(response.message.role)
                        .content(content.clone())
                        .build()?,
                );
                let chat = Chat::from((chat_id.to_string(), messages));
                let updated: Record = self.db.update(("chat", chat_id)).content(chat).await?;
                let chat_id = updated
                    .id
                    .to_string()
                    .split(':')
                    .last()
                    .unwrap_or_default()
                    .to_string();
                Ok(ChatResponse {
                    chat_id: chat_id.into(),
                    message: content.into(),
                })
            } else {
                Ok(ChatResponse {
                    chat_id: "none".into(),
                    message: "No response!".into(),
                })
            }
        } else {
            Ok(ChatResponse {
                chat_id: "none".into(),
                message: "No response!".into(),
            })
        }
    }

    pub async fn delete_chat(&self, chat_id: &str) -> anyhow::Result<()> {
        let _: Record = self.db.delete(("chat", chat_id)).await?;
        Ok(())
    }
}
