use crate::auth::AuthCacheEntry;
use crate::game_runtime::Command;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, mpsc};

#[derive(Clone)]
pub(crate) struct AppState {
    pub(crate) js_base: String,
    pub(crate) http: reqwest::Client,
    pub(crate) auth_cache: Arc<Mutex<HashMap<String, AuthCacheEntry>>>,
    pub(crate) runtime_tx: mpsc::Sender<Command>,
}
