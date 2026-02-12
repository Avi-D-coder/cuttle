use crate::api::handlers::GameStateResponse;
use crate::auth::AuthUser;
use crate::game_runtime::{GameAudience, GameStreamSubscription, RuntimeError};
use cutthroat_engine::Seat;
#[cfg(feature = "e2e-seed")]
use serde::{Deserialize, Serialize};
use tokio::sync::oneshot;

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
#[derive(Clone, Debug, Deserialize)]
pub(crate) struct SeedGameFromTranscriptInput {
    pub(crate) game_id: i64,
    pub(crate) players: Vec<SeedSeatInput>,
    pub(crate) dealer_seat: Seat,
    pub(crate) deck_tokens: Vec<String>,
    pub(crate) action_tokens: Vec<String>,
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

pub(crate) enum GameCommand {
    JoinGame {
        user: AuthUser,
        respond: oneshot::Sender<Result<Seat, RuntimeError>>,
    },
    LeaveGame {
        user: AuthUser,
        respond: oneshot::Sender<Result<(), RuntimeError>>,
    },
    SetReady {
        user: AuthUser,
        ready: bool,
        respond: oneshot::Sender<Result<(), RuntimeError>>,
    },
    StartGame {
        user: AuthUser,
        respond: oneshot::Sender<Result<(), RuntimeError>>,
    },
    GetState {
        user: AuthUser,
        spectate_intent: bool,
        respond: oneshot::Sender<Result<GameStateResponse, RuntimeError>>,
    },
    GetSpectateReplayState {
        user: AuthUser,
        game_state_index: i64,
        respond: oneshot::Sender<Result<GameStateResponse, RuntimeError>>,
    },
    SubscribeGameStream {
        user: AuthUser,
        spectate_intent: bool,
        respond: oneshot::Sender<Result<GameStreamSubscription, RuntimeError>>,
    },
    SocketDisconnected {
        user_id: i64,
        audience: GameAudience,
    },
    SyncRematchPresenceFromSource {
        disconnected_user_ids: Vec<i64>,
    },
    ApplyAction {
        user: AuthUser,
        expected_version: i64,
        action_tokens: String,
        respond: oneshot::Sender<Result<GameStateResponse, RuntimeError>>,
    },
    ToggleScrapStraighten {
        user: AuthUser,
        respond: oneshot::Sender<Result<(), RuntimeError>>,
    },
    EvaluateCleanup,
    Shutdown,
}
