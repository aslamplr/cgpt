use surrealdb::engine::local::Db;
use surrealdb::Surreal;

use crate::openai::Client;

pub struct AppState {
    pub(super) openai_client: Client,
    pub(super) db: Surreal<Db>,
}
