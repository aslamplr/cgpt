use std::sync::Arc;

use axum::{
    extract::{Json as JsonPayload, Path, State},
    http::StatusCode,
    Json,
};

use crate::services::{
    chat::{ChatHistory, ChatList, ChatRequest, ChatResponse},
    AppState,
};

pub async fn new_chat(
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

pub async fn get_chat(
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

pub async fn list_chat(State(state): State<Arc<AppState>>) -> Result<Json<ChatList>, StatusCode> {
    Ok(Json(
        state
            .list_chat()
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    ))
}

pub async fn continue_chat(
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

pub async fn delete_chat(
    State(state): State<Arc<AppState>>,
    Path(chat_id): Path<String>,
) -> Result<(), StatusCode> {
    state
        .delete_chat(&chat_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(())
}
