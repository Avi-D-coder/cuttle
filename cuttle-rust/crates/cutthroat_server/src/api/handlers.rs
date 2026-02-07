use crate::auth::{AuthUser, authorize};
use crate::game_runtime::{Command, GameStreamSubscription, LobbySnapshotInternal};
#[cfg(feature = "e2e-seed")]
use crate::game_runtime::{SeedGameInput, SeedGameResult, SeedSeatInput};
use crate::state::AppState;
use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
};
use cutthroat_engine::{Action, PublicView, Seat};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::{oneshot, watch};

#[derive(Deserialize)]
pub(crate) struct ActionRequest {
    pub(crate) expected_version: i64,
    pub(crate) action: Action,
}

#[derive(Deserialize)]
pub(crate) struct ReadyRequest {
    pub(crate) ready: bool,
}

#[derive(Serialize, Clone, Debug)]
pub(crate) struct LobbySummary {
    pub(crate) id: i64,
    pub(crate) name: String,
    pub(crate) seat_count: usize,
    pub(crate) ready_count: usize,
    pub(crate) status: i16,
}

#[derive(Serialize, Clone, Debug)]
pub(crate) struct SpectatableGameSummary {
    pub(crate) id: i64,
    pub(crate) name: String,
    pub(crate) seat_count: usize,
    pub(crate) status: i16,
    pub(crate) spectating_usernames: Vec<String>,
}

#[derive(Serialize, Clone, Debug)]
pub(crate) struct GameStateResponse {
    pub(crate) version: i64,
    pub(crate) seat: Seat,
    pub(crate) status: i16,
    pub(crate) player_view: PublicView,
    pub(crate) spectator_view: PublicView,
    pub(crate) legal_actions: Vec<Action>,
    pub(crate) lobby: LobbyView,
    pub(crate) log_tail: Vec<String>,
    pub(crate) tokenlog: String,
    pub(crate) is_spectator: bool,
    pub(crate) spectating_usernames: Vec<String>,
    pub(crate) scrap_straightened: bool,
}

#[derive(Serialize, Clone, Debug)]
pub(crate) struct LobbyView {
    pub(crate) seats: Vec<LobbySeatView>,
}

#[derive(Serialize)]
pub(crate) struct HealthResponse {
    pub(crate) alive: bool,
    pub(crate) service: &'static str,
    pub(crate) version: &'static str,
}

#[derive(Serialize, Clone, Debug)]
pub(crate) struct LobbySeatView {
    pub(crate) seat: Seat,
    pub(crate) user_id: i64,
    pub(crate) username: String,
    pub(crate) ready: bool,
}

#[cfg(feature = "e2e-seed")]
#[derive(Deserialize)]
pub(crate) struct SeedGameFromTokenlogRequest {
    pub(crate) game_id: i64,
    pub(crate) players: Vec<SeedSeatFromTokenlogRequest>,
    pub(crate) dealer_seat: Option<Seat>,
    pub(crate) tokenlog: String,
    pub(crate) status: Option<i16>,
    pub(crate) spectating_usernames: Option<Vec<String>>,
    pub(crate) name: Option<String>,
}

#[cfg(feature = "e2e-seed")]
#[derive(Deserialize)]
pub(crate) struct SeedSeatFromTokenlogRequest {
    pub(crate) seat: Seat,
    pub(crate) user_id: i64,
    pub(crate) username: String,
    pub(crate) ready: Option<bool>,
}

#[cfg(feature = "e2e-seed")]
#[derive(Serialize)]
pub(crate) struct SeedGameFromTokenlogResponse {
    pub(crate) game_id: i64,
    pub(crate) version: i64,
    pub(crate) status: i16,
    pub(crate) seat_user_ids: std::collections::BTreeMap<String, i64>,
    pub(crate) tokenlog: String,
    pub(crate) created: bool,
    pub(crate) replaced_existing: bool,
}

pub(crate) async fn get_health() -> Json<HealthResponse> {
    Json(HealthResponse {
        alive: true,
        service: "cutthroat",
        version: env!("CARGO_PKG_VERSION"),
    })
}

#[cfg(feature = "e2e-seed")]
pub(crate) async fn seed_game_from_tokenlog(
    State(state): State<AppState>,
    Json(body): Json<SeedGameFromTokenlogRequest>,
) -> Result<Json<SeedGameFromTokenlogResponse>, StatusCode> {
    let (tx, rx) = oneshot::channel();
    let seed = SeedGameInput {
        game_id: body.game_id,
        players: body
            .players
            .into_iter()
            .map(|seat| SeedSeatInput {
                seat: seat.seat,
                user_id: seat.user_id,
                username: seat.username,
                ready: seat.ready,
            })
            .collect(),
        dealer_seat: body.dealer_seat,
        tokenlog: body.tokenlog,
        status: body.status,
        spectating_usernames: body.spectating_usernames,
        name: body.name,
    };
    state
        .runtime_tx
        .send(Command::SeedGameFromTokenlog { seed, respond: tx })
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let SeedGameResult {
        game_id,
        version,
        status,
        seat_user_ids,
        tokenlog,
        created,
        replaced_existing,
    } = rx
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .map_err(|err| err.status_code())?;

    Ok(Json(SeedGameFromTokenlogResponse {
        game_id,
        version,
        status,
        seat_user_ids,
        tokenlog,
        created,
        replaced_existing,
    }))
}

pub(crate) async fn create_game(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let user = authorize(&state, &headers).await?;
    let (tx, rx) = oneshot::channel();
    state
        .runtime_tx
        .send(Command::CreateGame { user, respond: tx })
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let id = rx.await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(serde_json::json!({ "id": id })))
}

pub(crate) async fn join_game(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let user = authorize(&state, &headers).await?;
    let (tx, rx) = oneshot::channel();
    state
        .runtime_tx
        .send(Command::JoinGame {
            game_id: id,
            user,
            respond: tx,
        })
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let seat = rx
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .map_err(|err| err.status_code())?;
    Ok(Json(serde_json::json!({ "seat": seat })))
}

pub(crate) async fn rematch_game(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let user = authorize(&state, &headers).await?;
    let (tx, rx) = oneshot::channel();
    state
        .runtime_tx
        .send(Command::RematchGame {
            game_id: id,
            user,
            respond: tx,
        })
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let rematch_id = rx
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .map_err(|err| err.status_code())?;
    Ok(Json(serde_json::json!({ "id": rematch_id })))
}

pub(crate) async fn leave_game(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    headers: HeaderMap,
) -> Result<StatusCode, StatusCode> {
    let user = authorize(&state, &headers).await?;
    let (tx, rx) = oneshot::channel();
    state
        .runtime_tx
        .send(Command::LeaveGame {
            game_id: id,
            user,
            respond: tx,
        })
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    rx.await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .map_err(|err| err.status_code())?;
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn set_ready(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    headers: HeaderMap,
    Json(body): Json<ReadyRequest>,
) -> Result<StatusCode, StatusCode> {
    let user = authorize(&state, &headers).await?;
    let (tx, rx) = oneshot::channel();
    state
        .runtime_tx
        .send(Command::SetReady {
            game_id: id,
            user,
            ready: body.ready,
            respond: tx,
        })
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    rx.await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .map_err(|err| err.status_code())?;
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn start_game(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    headers: HeaderMap,
) -> Result<StatusCode, StatusCode> {
    let _user = authorize(&state, &headers).await?;
    let (tx, rx) = oneshot::channel();
    state
        .runtime_tx
        .send(Command::StartGame {
            game_id: id,
            respond: tx,
        })
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    rx.await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .map_err(|err| err.status_code())?;
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn get_state(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    headers: HeaderMap,
) -> Result<Json<GameStateResponse>, StatusCode> {
    get_state_inner(state, id, headers, false).await
}

pub(crate) async fn get_spectate_state(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    headers: HeaderMap,
) -> Result<Json<GameStateResponse>, StatusCode> {
    get_state_inner(state, id, headers, true).await
}

async fn get_state_inner(
    state: AppState,
    id: i64,
    headers: HeaderMap,
    spectate_intent: bool,
) -> Result<Json<GameStateResponse>, StatusCode> {
    let user = authorize(&state, &headers).await?;
    let (tx, rx) = oneshot::channel();
    state
        .runtime_tx
        .send(Command::GetState {
            game_id: id,
            user,
            spectate_intent,
            respond: tx,
        })
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let resp = rx
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .map_err(|err| err.status_code())?;
    Ok(Json(resp))
}

pub(crate) async fn post_action(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    headers: HeaderMap,
    Json(body): Json<ActionRequest>,
) -> Result<Json<GameStateResponse>, StatusCode> {
    let user = authorize(&state, &headers).await?;
    let (tx, rx) = oneshot::channel();
    state
        .runtime_tx
        .send(Command::ApplyAction {
            game_id: id,
            user,
            expected_version: body.expected_version,
            action: body.action,
            respond: tx,
        })
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let resp = rx
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .map_err(|err| err.status_code())?;
    Ok(Json(resp))
}

pub(crate) async fn apply_action_internal(
    state: &AppState,
    game_id: i64,
    user: AuthUser,
    expected_version: i64,
    action: Action,
) -> Result<(), (u16, String)> {
    let (tx, rx) = oneshot::channel();
    state
        .runtime_tx
        .send(Command::ApplyAction {
            game_id,
            user,
            expected_version,
            action,
            respond: tx,
        })
        .await
        .map_err(|_| (500, "server error".to_string()))?;

    rx.await
        .map_err(|_| (500, "server error".to_string()))?
        .map(|_| ())
        .map_err(|err| (err.code(), err.message()))
}

pub(crate) async fn toggle_scrap_straighten_internal(
    state: &AppState,
    game_id: i64,
    user: AuthUser,
) -> Result<(), (u16, String)> {
    let (tx, rx) = oneshot::channel();
    state
        .runtime_tx
        .send(Command::ToggleScrapStraighten {
            game_id,
            user,
            respond: tx,
        })
        .await
        .map_err(|_| (500, "server error".to_string()))?;

    rx.await
        .map_err(|_| (500, "server error".to_string()))?
        .map(|_| ())
        .map_err(|err| (err.code(), err.message()))
}

pub(crate) async fn subscribe_game_stream(
    state: &AppState,
    game_id: i64,
    user: AuthUser,
    spectate_intent: bool,
) -> Result<GameStreamSubscription, (u16, String)> {
    let (tx, rx) = oneshot::channel();
    state
        .runtime_tx
        .send(Command::SubscribeGameStream {
            game_id,
            user,
            spectate_intent,
            respond: tx,
        })
        .await
        .map_err(|_| (500, "server error".to_string()))?;

    rx.await
        .map_err(|_| (500, "server error".to_string()))?
        .map_err(|err| (err.code(), err.message()))
}

pub(crate) async fn subscribe_lobby_stream(
    state: &AppState,
) -> Result<watch::Receiver<Arc<LobbySnapshotInternal>>, StatusCode> {
    let (tx, rx) = oneshot::channel();
    state
        .runtime_tx
        .send(Command::SubscribeLobbyStream { respond: tx })
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    rx.await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

pub(crate) async fn set_spectator_disconnected(state: &AppState, game_id: i64, user_id: i64) {
    let _ = state
        .runtime_tx
        .send(Command::SpectatorDisconnected { game_id, user_id })
        .await;
}
