use crate::api::handlers::GameStateResponse;
use crate::auth::AuthUser;
use crate::game_runtime::{GameStreamSubscription, LobbySnapshotInternal, RuntimeError};
use cutthroat_engine::{Action, Seat};
#[cfg(feature = "e2e-seed")]
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::{oneshot, watch};

#[cfg(feature = "e2e-seed")]
#[derive(Clone, Debug, Deserialize)]
pub(crate) struct SeedSeatInput {
    pub(crate) seat: Seat,
    pub(crate) user_id: i64,
    pub(crate) username: String,
    pub(crate) ready: Option<bool>,
}

#[cfg(feature = "e2e-seed")]
#[derive(Clone, Debug, Deserialize)]
pub(crate) struct SeedGameInput {
    pub(crate) game_id: i64,
    pub(crate) players: Vec<SeedSeatInput>,
    pub(crate) dealer_seat: Option<Seat>,
    pub(crate) tokenlog: String,
    pub(crate) status: Option<i16>,
    pub(crate) spectating_usernames: Option<Vec<String>>,
    pub(crate) name: Option<String>,
}

#[cfg(feature = "e2e-seed")]
#[derive(Clone, Debug, Serialize)]
pub(crate) struct SeedGameResult {
    pub(crate) game_id: i64,
    pub(crate) version: i64,
    pub(crate) status: i16,
    pub(crate) seat_user_ids: std::collections::BTreeMap<String, i64>,
    pub(crate) tokenlog: String,
    pub(crate) created: bool,
    pub(crate) replaced_existing: bool,
}

pub(crate) enum Command {
    CreateGame {
        user: AuthUser,
        respond: oneshot::Sender<i64>,
    },
    JoinGame {
        game_id: i64,
        user: AuthUser,
        respond: oneshot::Sender<Result<Seat, RuntimeError>>,
    },
    LeaveGame {
        game_id: i64,
        user: AuthUser,
        respond: oneshot::Sender<Result<(), RuntimeError>>,
    },
    RematchGame {
        game_id: i64,
        user: AuthUser,
        respond: oneshot::Sender<Result<i64, RuntimeError>>,
    },
    SetReady {
        game_id: i64,
        user: AuthUser,
        ready: bool,
        respond: oneshot::Sender<Result<(), RuntimeError>>,
    },
    StartGame {
        game_id: i64,
        user: AuthUser,
        respond: oneshot::Sender<Result<(), RuntimeError>>,
    },
    GetState {
        game_id: i64,
        user: AuthUser,
        spectate_intent: bool,
        respond: oneshot::Sender<Result<GameStateResponse, RuntimeError>>,
    },
    GetSpectateReplayState {
        game_id: i64,
        user: AuthUser,
        game_state_index: i64,
        respond: oneshot::Sender<Result<GameStateResponse, RuntimeError>>,
    },
    SubscribeGameStream {
        game_id: i64,
        user: AuthUser,
        spectate_intent: bool,
        respond: oneshot::Sender<Result<GameStreamSubscription, RuntimeError>>,
    },
    SubscribeLobbyStream {
        respond: oneshot::Sender<watch::Receiver<Arc<LobbySnapshotInternal>>>,
    },
    SpectatorDisconnected {
        game_id: i64,
        user_id: i64,
    },
    ApplyAction {
        game_id: i64,
        user: AuthUser,
        expected_version: i64,
        action: Action,
        respond: oneshot::Sender<Result<GameStateResponse, RuntimeError>>,
    },
    ToggleScrapStraighten {
        game_id: i64,
        user: AuthUser,
        respond: oneshot::Sender<Result<(), RuntimeError>>,
    },
    #[cfg(feature = "e2e-seed")]
    SeedGameFromTokenlog {
        seed: SeedGameInput,
        respond: oneshot::Sender<Result<SeedGameResult, RuntimeError>>,
    },
}
