use std::sync::Arc;

use anyhow::Context;
use axum::routing::{delete, put};
use axum::{
    routing::{get, post},
    Router,
};
use surrealdb::engine::local::File;
use surrealdb::Surreal;

use cgpt_api::handlers::chat::{continue_chat, delete_chat, get_chat, list_chat, new_chat};
use cgpt_api::openai::Client;
use cgpt_api::services::AppState;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    if std::env::var("OPENAI_API_KEY").is_err() {
        dotenv::dotenv()
            .with_context(|| "Set OPENAI_API_KEY environment variable or add in .env file")?;
    }

    let openai_client = Client::new();
    let db = Surreal::new::<File>("/tmp/cgpt.rocks.db").await?;
    db.use_ns("cgpt").use_db("default").await?;

    let shared_state = Arc::new(AppState::new(openai_client, db));

    let app = Router::new()
        .route("/", get(|| async { "cgpt REST api service!" }))
        .route("/api/chat", get(list_chat))
        .route("/api/chat", post(new_chat))
        .route("/api/chat/:chat_id", get(get_chat))
        .route("/api/chat/:chat_id", put(continue_chat))
        .route("/api/chat/:chat_id", delete(delete_chat))
        .with_state(shared_state);

    axum::Server::bind(&"0.0.0.0:3000".parse()?)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}
