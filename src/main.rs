use std::{borrow::Cow, ops::Deref, sync::Arc};

use anyhow::Context;
use async_openai::{
    config::OpenAIConfig,
    types::{
        ChatCompletionRequestMessage, ChatCompletionRequestMessageArgs,
        CreateChatCompletionRequestArgs, CreateChatCompletionResponse, Role,
    },
    Client,
};
use axum::{
    extract::{Json as JsonPayload, Path, State},
    http::StatusCode,
    routing::{delete, put},
};
use axum::{
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use surrealdb::Surreal;
use surrealdb::{
    engine::local::{Db, File},
    sql::Thing,
};

struct AppState {
    openai_client: async_openai::Client<OpenAIConfig>,
    db: Surreal<Db>,
}

#[derive(Deserialize)]
struct ChatRequest {
    message: Cow<'static, str>,
}

#[derive(Serialize)]
struct ChatResponse {
    chat_id: Cow<'static, str>,
    message: Cow<'static, str>,
}

#[derive(Serialize)]
struct ChatHistory {
    chat_id: Cow<'static, str>,
    messages: Vec<Cow<'static, str>>,
}

#[derive(Serialize)]
struct ChatList {
    chats: Vec<Cow<'static, str>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Chat {
    chat_id: Cow<'static, str>,
    messages: Vec<ChatMessage>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ChatMessage(ChatCompletionRequestMessage);

#[derive(Debug, Deserialize)]
struct Record {
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
    async fn new_chat(&self, message: &str) -> anyhow::Result<ChatResponse> {
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

        let response = openai_chat(&self.openai_client, &messages).await?;

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

    async fn list_chat(&self) -> anyhow::Result<ChatList> {
        let chats: Vec<Chat> = self.db.select("chat").await?;
        let chats = chats
            .into_iter()
            .map(|c| c.chat_id)
            .collect::<Vec<Cow<'static, str>>>();
        Ok(ChatList { chats })
    }

    async fn get_chat(&self, chat_id: &str) -> anyhow::Result<ChatHistory> {
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

    async fn continue_chat(&self, chat_id: &str, message: &str) -> anyhow::Result<ChatResponse> {
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

            let response = openai_chat(&self.openai_client, &messages).await?;

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

    async fn delete_chat(&self, chat_id: &str) -> anyhow::Result<()> {
        self.db.delete(("chat", chat_id)).await?;
        Ok(())
    }
}

async fn new_chat(
    State(state): State<Arc<AppState>>,
    JsonPayload(payload): JsonPayload<ChatRequest>,
) -> Result<Json<ChatResponse>, StatusCode> {
    Ok(Json(
        state
            .new_chat(&payload.message)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    ))
}

async fn get_chat(
    State(state): State<Arc<AppState>>,
    Path(chat_id): Path<String>,
) -> Result<Json<ChatHistory>, StatusCode> {
    Ok(Json(
        state
            .get_chat(&chat_id)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    ))
}

async fn list_chat(State(state): State<Arc<AppState>>) -> Result<Json<ChatList>, StatusCode> {
    Ok(Json(
        state
            .list_chat()
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    ))
}

async fn continue_chat(
    State(state): State<Arc<AppState>>,
    Path(chat_id): Path<String>,
    JsonPayload(payload): JsonPayload<ChatRequest>,
) -> Result<Json<ChatResponse>, StatusCode> {
    Ok(Json(
        state
            .continue_chat(&chat_id, &payload.message)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    ))
}

async fn delete_chat(
    State(state): State<Arc<AppState>>,
    Path(chat_id): Path<String>,
) -> Result<(), StatusCode> {
    state
        .delete_chat(&chat_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    if std::env::var("OPENAI_API_KEY").is_err() {
        dotenv::dotenv()
            .with_context(|| "Set OPENAI_API_KEY environment variable or add in .env file")?;
    }

    let openai_client = Client::new();
    let db = Surreal::new::<File>("/tmp/cgpt.rocks.db").await?;
    db.use_ns("cgpt").use_db("default").await?;

    let shared_state = Arc::new(AppState { openai_client, db });

    let app = Router::new()
        .route("/", get(|| async { "cgpt REST api service!" }))
        .route("/chat", get(list_chat))
        .route("/chat", post(new_chat))
        .route("/chat/:chat_id", get(get_chat))
        .route("/chat/:chat_id", put(continue_chat))
        .route("/chat/:chat_id", delete(delete_chat))
        .with_state(shared_state);

    axum::Server::bind(&"0.0.0.0:3000".parse()?)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}

use rand::Rng;

fn generate_chat_id() -> String {
    const LENGTH: usize = 16;
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ\
                             abcdefghijklmnopqrstuvwxyz\
                             0123456789";
    let mut rng = rand::thread_rng();
    (0..LENGTH)
        .map(|_| CHARSET[rng.gen_range(0..CHARSET.len())] as char)
        .collect()
}

async fn openai_chat(
    client: &Client<OpenAIConfig>,
    messages: &[ChatCompletionRequestMessage],
) -> anyhow::Result<CreateChatCompletionResponse> {
    let request = CreateChatCompletionRequestArgs::default()
        .max_tokens(512u16)
        .model("gpt-3.5-turbo")
        .messages(messages)
        .build()?;

    Ok(client.chat().create(request).await?)
}
