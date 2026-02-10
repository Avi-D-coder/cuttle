use crate::auth::AuthCacheEntry;
use crate::game_runtime::GlobalRuntimeState;
use crate::persistence::CompletedGameRecord;
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock, mpsc};

#[derive(Clone)]
pub(crate) struct AppState {
    pub(crate) js_base: String,
    pub(crate) http: reqwest::Client,
    pub(crate) db: Option<PgPool>,
    pub(crate) auth_cache: Arc<Mutex<HashMap<String, AuthCacheEntry>>>,
    pub(crate) runtime: Arc<RwLock<GlobalRuntimeState>>,
    pub(crate) persistence_tx: mpsc::Sender<CompletedGameRecord>,
}
