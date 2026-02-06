use crate::api::handlers::{GameStateResponse, LobbyListsResponse};
use crate::auth::AuthUser;
use crate::state::ScrapStraightenUpdate;
use crate::store::StoreError;
use cutthroat_engine::{Action, Seat};
use tokio::sync::oneshot;

#[derive(Debug)]
pub(crate) enum Command {
    CreateGame {
        user: AuthUser,
        respond: oneshot::Sender<i64>,
    },
    JoinGame {
        game_id: i64,
        user: AuthUser,
        respond: oneshot::Sender<Result<Seat, StoreError>>,
    },
    LeaveGame {
        game_id: i64,
        user: AuthUser,
        respond: oneshot::Sender<Result<(), StoreError>>,
    },
    RematchGame {
        game_id: i64,
        user: AuthUser,
        respond: oneshot::Sender<Result<i64, StoreError>>,
    },
    SetReady {
        game_id: i64,
        user: AuthUser,
        ready: bool,
        respond: oneshot::Sender<Result<(), StoreError>>,
    },
    StartGame {
        game_id: i64,
        respond: oneshot::Sender<Result<(), StoreError>>,
    },
    GetState {
        game_id: i64,
        user: AuthUser,
        spectate_intent: bool,
        respond: oneshot::Sender<Result<GameStateResponse, StoreError>>,
    },
    ValidateViewer {
        game_id: i64,
        user: AuthUser,
        spectate_intent: bool,
        respond: oneshot::Sender<Result<(), StoreError>>,
    },
    SpectatorConnected {
        game_id: i64,
        user: AuthUser,
        respond: oneshot::Sender<Result<(), StoreError>>,
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
        respond: oneshot::Sender<Result<GameStateResponse, StoreError>>,
    },
    ToggleScrapStraighten {
        game_id: i64,
        user: AuthUser,
        respond: oneshot::Sender<Result<ScrapStraightenUpdate, StoreError>>,
    },
    GetLobbyListForUser {
        user_id: i64,
        respond: oneshot::Sender<LobbyListsResponse>,
    },
}
