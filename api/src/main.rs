use std::sync::Arc;

use anyhow::Context;
use axum::http::header::{ACCEPT, ACCEPT_ENCODING, AUTHORIZATION, CONTENT_TYPE, ORIGIN};
use axum::{routing::get, Router};
use surrealdb::engine::remote::ws::Ws;
use surrealdb::opt::auth::Root;
use surrealdb::Surreal;
use tower_http::compression::CompressionLayer;
use tower_http::cors::CorsLayer;
use tower_http::trace::{self, TraceLayer};
use tracing::Level;

use cgpt_api::handlers::chat::{continue_chat, delete_chat, get_chat, list_chat, new_chat};
use cgpt_api::openai::Client;
use cgpt_api::services::AppState;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialise environment variables from .env file
    dotenv::dotenv().with_context(|| "Set required environment variables in .env file")?;

    // Initialise tracing for logging
    let filter = std::env::var("RUST_LOG")
        .unwrap_or_else(|_| "cgpt_api=debug,towe_http=info,info".to_owned());
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .without_time()
        .compact()
        .init();

    // Initialise SurrealDB database client and signin
    let db_url = std::env::var("DB_URL").unwrap_or_else(|_| "localhost:8000".to_owned());
    let db_username = std::env::var("DB_USERNAME").unwrap_or_else(|_| "root".to_owned());
    let db_password = std::env::var("DB_PASSWORD").unwrap_or_else(|_| "root".to_owned());

    let db = Surreal::new::<Ws>(db_url.as_str()).await?;
    db.use_ns("cgpt").use_db("default_database").await?;

    db.signin(Root {
        username: &db_username,
        password: &db_password,
    })
    .await?;

    // Initialise OpenAI client
    let openai_client = Client::new();

    // Initialise shared state for axum
    let shared_state = Arc::new(AppState::new(openai_client, db));

    // Setup axum app
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
        .layer(
            CorsLayer::new()
                .allow_headers(vec![
                    ACCEPT,
                    ACCEPT_ENCODING,
                    AUTHORIZATION,
                    CONTENT_TYPE,
                    ORIGIN,
                ])
                .allow_methods(tower_http::cors::Any)
                .allow_origin(tower_http::cors::Any),
        )
        .layer(CompressionLayer::new().gzip(true).deflate(true))
        .with_state(shared_state);

    #[cfg(feature = "lambda")]
    {
        // Start axum server within lambda runtime
        let app = tower::ServiceBuilder::new()
            .layer(axum_aws_lambda::LambdaLayer::default())
            .service(app);

        lambda_http::run(app).await.expect("failed to run lambda!");
    }

    #[cfg(not(feature = "lambda"))]
    {
        // Start axum server outside lambda runtime
        axum::Server::bind(&"0.0.0.0:3000".parse()?)
            .serve(app.into_make_service())
            .await?;
    }

    Ok(())
}
