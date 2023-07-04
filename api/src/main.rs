use std::sync::Arc;

use anyhow::Context;
use axum::{routing::get, Router};
use surrealdb::engine::remote::ws::Ws;
use surrealdb::opt::auth::Root;
use surrealdb::Surreal;
use tower_http::trace::{self, TraceLayer};
use tracing::Level;

use cgpt_api::handlers::chat::{continue_chat, delete_chat, get_chat, list_chat, new_chat};
use cgpt_api::openai::Client;
use cgpt_api::services::AppState;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    if std::env::var("OPENAI_API_KEY").is_err() {
        dotenv::dotenv()
            .with_context(|| "Set OPENAI_API_KEY environment variable or add in .env file")?;
    }
    let filter = std::env::var("RUST_LOG")
        .unwrap_or_else(|_| "cgpt_api=debug,towe_http=info,info".to_owned());
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .compact()
        .init();

    let openai_client = Client::new();
    let db = Surreal::new::<Ws>("localhost:8000").await?;
    db.use_ns("cgpt").use_db("default_database").await?;

    db.signin(Root {
        username: "root",
        password: "root",
    })
    .await?;

    let shared_state = Arc::new(AppState::new(openai_client, db));

    let app = Router::new()
        .route("/", get(|| async { "cgpt REST api service!" }))
        .route("/chat", get(list_chat).post(new_chat))
        .route(
            "/chat/:chat_id",
            get(get_chat).put(continue_chat).delete(delete_chat),
        )
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(trace::DefaultMakeSpan::new().level(Level::INFO))
                .on_response(trace::DefaultOnResponse::new().level(Level::INFO)),
        )
        .with_state(shared_state);

    axum::Server::bind(&"0.0.0.0:3000".parse()?)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}
