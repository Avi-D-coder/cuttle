use crate::auth::AuthCacheEntry;
use crate::store::Command;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, broadcast, mpsc};

#[derive(Clone)]
pub(crate) struct AppState {
    pub(crate) js_base: String,
    pub(crate) http: reqwest::Client,
    pub(crate) updates: broadcast::Sender<GameUpdate>,
    pub(crate) lobby_updates: broadcast::Sender<LobbyListUpdate>,
    pub(crate) scrap_straighten_updates: broadcast::Sender<ScrapStraightenUpdate>,
    pub(crate) auth_cache: Arc<Mutex<HashMap<String, AuthCacheEntry>>>,
    pub(crate) store_tx: mpsc::Sender<Command>,
}

#[derive(Clone, Debug)]
pub(crate) struct GameUpdate {
    pub(crate) game_id: i64,
}

#[derive(Clone, Debug)]
pub(crate) struct LobbyListUpdate {
    pub(crate) _changed: bool,
}

#[derive(Clone, Debug)]
pub(crate) struct ScrapStraightenUpdate {
    pub(crate) game_id: i64,
    pub(crate) straightened: bool,
    pub(crate) actor_seat: cutthroat_engine::Seat,
}
