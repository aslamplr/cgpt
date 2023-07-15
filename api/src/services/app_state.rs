use surrealdb::engine::remote::ws::Client as WsClient;
use surrealdb::Surreal;

use crate::openai::Client as OpenAiClient;

pub struct AppState {
    pub(super) openai_client: OpenAiClient,
    pub(super) db: Surreal<WsClient>,
}
