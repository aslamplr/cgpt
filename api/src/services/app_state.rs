use surrealdb::engine::remote::ws::Client as Ws;
use surrealdb::Surreal;

use crate::openai::Client;

pub struct AppState {
    pub(super) openai_client: Client,
    pub(super) db: Surreal<Ws>,
}
