mod persistence;

use axum::extract::ws::{Message, WebSocket};
use axum::{
    Json, Router,
    extract::{Path, State, WebSocketUpgrade},
    http::{HeaderMap, StatusCode, header},
    response::IntoResponse,
    routing::{get, post},
};
use chrono::{DateTime, Utc};
use cutthroat_engine::state::{PhaseView, PublicCard};
use cutthroat_engine::{
    Action, Card, CutthroatState, LastEventView, OneOffTarget, Phase, PublicView, Seat, SevenPlay,
    Winner, append_action, encode_header, parse_tokenlog,
};
use persistence::{CompletedGameRecord, ensure_schema_ready, run_persistence_worker};
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgPoolOptions;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, broadcast, mpsc, oneshot};
use tracing::{error, info};

const STATUS_LOBBY: i16 = 0;
const STATUS_STARTED: i16 = 1;
const STATUS_FINISHED: i16 = 2;
const AUTH_CACHE_TTL: Duration = Duration::from_secs(30);
const LOG_TAIL_LIMIT: usize = 60;

#[derive(Clone)]
struct AppState {
    js_base: String,
    http: reqwest::Client,
    updates: broadcast::Sender<GameUpdate>,
    lobby_updates: broadcast::Sender<LobbyListUpdate>,
    scrap_straighten_updates: broadcast::Sender<ScrapStraightenUpdate>,
    auth_cache: Arc<Mutex<HashMap<String, AuthCacheEntry>>>,
    store_tx: mpsc::Sender<Command>,
}

#[derive(Clone, Debug)]
struct GameUpdate {
    game_id: i64,
}

#[derive(Clone, Debug)]
struct LobbyListUpdate {
    _changed: bool,
}

#[derive(Clone, Debug)]
struct ScrapStraightenUpdate {
    game_id: i64,
    straightened: bool,
    actor_seat: Seat,
}

#[derive(Clone, Debug)]
struct AuthUser {
    id: i64,
    username: String,
}

#[derive(Clone, Debug)]
struct AuthCacheEntry {
    user: AuthUser,
    expires_at: Instant,
}

#[derive(Deserialize)]
struct ActionRequest {
    expected_version: i64,
    action: Action,
}

#[derive(Deserialize)]
struct ReadyRequest {
    ready: bool,
}

#[derive(Serialize, Clone, Debug)]
struct LobbySummary {
    id: i64,
    name: String,
    seat_count: usize,
    ready_count: usize,
    status: i16,
}

#[derive(Serialize, Clone, Debug)]
struct SpectatableGameSummary {
    id: i64,
    name: String,
    seat_count: usize,
    status: i16,
    spectating_usernames: Vec<String>,
}

#[derive(Serialize, Clone, Debug)]
struct LobbyListsResponse {
    lobbies: Vec<LobbySummary>,
    spectatable_games: Vec<SpectatableGameSummary>,
}

#[derive(Serialize, Debug)]
struct GameStateResponse {
    version: i64,
    seat: Seat,
    status: i16,
    player_view: PublicView,
    spectator_view: PublicView,
    legal_actions: Vec<Action>,
    lobby: LobbyView,
    log_tail: Vec<String>,
    tokenlog: String,
    is_spectator: bool,
    spectating_usernames: Vec<String>,
}

#[derive(Serialize, Debug)]
struct LobbyView {
    seats: Vec<LobbySeatView>,
}

#[derive(Serialize)]
struct HealthResponse {
    alive: bool,
    service: &'static str,
    version: &'static str,
}

#[derive(Serialize, Debug)]
struct LobbySeatView {
    seat: Seat,
    user_id: i64,
    username: String,
    ready: bool,
}

#[derive(Deserialize)]
#[serde(tag = "type")]
enum WsClientMessage {
    #[serde(rename = "action")]
    Action {
        expected_version: i64,
        action: Action,
    },
    #[serde(rename = "scrap_straighten")]
    ScrapStraighten,
}

#[derive(Serialize)]
#[serde(tag = "type")]
enum WsServerMessage {
    #[serde(rename = "state")]
    State(Box<GameStateResponse>),
    #[serde(rename = "lobbies")]
    Lobbies {
        lobbies: Vec<LobbySummary>,
        spectatable_games: Vec<SpectatableGameSummary>,
    },
    #[serde(rename = "scrap_straighten")]
    ScrapStraighten {
        game_id: i64,
        straightened: bool,
        actor_seat: Seat,
    },
    #[serde(rename = "error")]
    Error { code: u16, message: String },
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let js_base = std::env::var("JS_INTERNAL_BASE_URL")
        .unwrap_or_else(|_| "http://localhost:1337".to_string());
    let bind_addr = std::env::var("RUST_BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:4000".to_string());
    let database_url = resolve_database_url()?;
    let auto_run_migrations = resolve_auto_run_migrations();
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;
    ensure_schema_ready(&pool, auto_run_migrations).await?;

    let http = reqwest::Client::new();
    let (updates, _) = broadcast::channel(128);
    let (lobby_updates, _) = broadcast::channel(128);
    let (scrap_straighten_updates, _) = broadcast::channel(128);
    let auth_cache = Arc::new(Mutex::new(HashMap::new()));
    let (store_tx, store_rx) = mpsc::channel(256);
    let (persistence_tx, persistence_rx) = mpsc::channel(256);

    tokio::spawn(store_task(
        store_rx,
        persistence_tx,
        updates.clone(),
        lobby_updates.clone(),
        scrap_straighten_updates.clone(),
    ));
    tokio::spawn(run_persistence_worker(persistence_rx, pool));

    let state = AppState {
        js_base,
        http,
        updates,
        lobby_updates,
        scrap_straighten_updates,
        auth_cache,
        store_tx,
    };

    let app = Router::new()
        .route("/cutthroat/api/v1/health", get(get_health))
        .route("/cutthroat/api/v1/games", post(create_game))
        .route("/cutthroat/api/v1/games/{id}/join", post(join_game))
        .route("/cutthroat/api/v1/games/{id}/leave", post(leave_game))
        .route("/cutthroat/api/v1/games/{id}/rematch", post(rematch_game))
        .route("/cutthroat/api/v1/games/{id}/ready", post(set_ready))
        .route("/cutthroat/api/v1/games/{id}/start", post(start_game))
        .route("/cutthroat/api/v1/games/{id}/state", get(get_state))
        .route(
            "/cutthroat/api/v1/games/{id}/spectate/state",
            get(get_spectate_state),
        )
        .route("/cutthroat/api/v1/games/{id}/action", post(post_action))
        .route("/cutthroat/ws/games/{id}", get(ws_handler))
        .route(
            "/cutthroat/ws/games/{id}/spectate",
            get(ws_spectate_handler),
        )
        .route("/cutthroat/ws/lobbies", get(ws_lobbies_handler))
        .with_state(state);

    info!("cutthroat server listening on {}", bind_addr);
    let listener = tokio::net::TcpListener::bind(bind_addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

fn resolve_database_url() -> Result<String, anyhow::Error> {
    resolve_database_url_from(
        std::env::var("CUTTHROAT_DATABASE_URL").ok(),
        std::env::var("DATABASE_URL").ok(),
    )
}

fn resolve_auto_run_migrations() -> bool {
    resolve_auto_run_migrations_from(std::env::var("CUTTHROAT_AUTO_RUN_MIGRATIONS").ok())
}

fn resolve_auto_run_migrations_from(value: Option<String>) -> bool {
    value
        .as_deref()
        .map(|raw| raw.trim().eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

fn resolve_database_url_from(
    cutthroat_database_url: Option<String>,
    database_url: Option<String>,
) -> Result<String, anyhow::Error> {
    cutthroat_database_url
        .or(database_url)
        .ok_or_else(|| {
            anyhow::anyhow!(
                "Missing database URL for cutthroat persistence. Set `CUTTHROAT_DATABASE_URL` (preferred) or `DATABASE_URL`."
            )
        })
}

async fn get_health() -> Json<HealthResponse> {
    Json(HealthResponse {
        alive: true,
        service: "cutthroat",
        version: env!("CARGO_PKG_VERSION"),
    })
}

async fn create_game(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let user = authorize(&state, &headers).await?;
    let (tx, rx) = oneshot::channel();
    state
        .store_tx
        .send(Command::CreateGame { user, respond: tx })
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let id = rx.await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(serde_json::json!({ "id": id })))
}

async fn join_game(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let user = authorize(&state, &headers).await?;
    let (tx, rx) = oneshot::channel();
    state
        .store_tx
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

async fn rematch_game(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let user = authorize(&state, &headers).await?;
    let (tx, rx) = oneshot::channel();
    state
        .store_tx
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

async fn leave_game(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    headers: HeaderMap,
) -> Result<StatusCode, StatusCode> {
    let user = authorize(&state, &headers).await?;
    let (tx, rx) = oneshot::channel();
    state
        .store_tx
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

async fn set_ready(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    headers: HeaderMap,
    Json(body): Json<ReadyRequest>,
) -> Result<StatusCode, StatusCode> {
    let user = authorize(&state, &headers).await?;
    let (tx, rx) = oneshot::channel();
    state
        .store_tx
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

async fn start_game(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    headers: HeaderMap,
) -> Result<StatusCode, StatusCode> {
    let _user = authorize(&state, &headers).await?;
    let (tx, rx) = oneshot::channel();
    state
        .store_tx
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

async fn get_state(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    headers: HeaderMap,
) -> Result<Json<GameStateResponse>, StatusCode> {
    get_state_inner(state, id, headers, false).await
}

async fn get_spectate_state(
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
        .store_tx
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

async fn post_action(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    headers: HeaderMap,
    Json(body): Json<ActionRequest>,
) -> Result<Json<GameStateResponse>, StatusCode> {
    let user = authorize(&state, &headers).await?;
    let (tx, rx) = oneshot::channel();
    state
        .store_tx
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

async fn ws_handler(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    headers: HeaderMap,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    let user = match authorize(&state, &headers).await {
        Ok(user) => user,
        Err(code) => return code.into_response(),
    };

    ws.on_upgrade(move |socket| handle_ws(socket, state, id, user, false))
}

async fn ws_spectate_handler(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    headers: HeaderMap,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    let user = match authorize(&state, &headers).await {
        Ok(user) => user,
        Err(code) => return code.into_response(),
    };

    ws.on_upgrade(move |socket| handle_ws(socket, state, id, user, true))
}

async fn ws_lobbies_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    let user = match authorize(&state, &headers).await {
        Ok(user) => user,
        Err(code) => return code.into_response(),
    };

    ws.on_upgrade(move |socket| handle_lobbies_ws(socket, state, user))
}

async fn handle_ws(
    mut socket: WebSocket,
    state: AppState,
    game_id: i64,
    user: AuthUser,
    spectate_intent: bool,
) {
    let mut updates = state.updates.subscribe();
    let mut scrap_straighten_updates = state.scrap_straighten_updates.subscribe();
    let mut spectator_registered = false;
    if let Err((code, message)) =
        validate_viewer(&state, game_id, user.clone(), spectate_intent).await
    {
        let _ = socket
            .send(Message::Text(
                serde_json::to_string(&WsServerMessage::Error { code, message })
                    .unwrap()
                    .into(),
            ))
            .await;
        return;
    }

    match build_state_response_for_user(&state, game_id, user.clone(), spectate_intent).await {
        Ok(resp) => {
            if resp.is_spectator {
                if set_spectator_connected(&state, game_id, user.clone())
                    .await
                    .is_err()
                {
                    return;
                }
                spectator_registered = true;
            }
            if socket
                .send(Message::Text(
                    serde_json::to_string(&WsServerMessage::State(Box::new(resp)))
                        .unwrap()
                        .into(),
                ))
                .await
                .is_err()
            {
                if spectator_registered {
                    set_spectator_disconnected(&state, game_id, user.id).await;
                }
                return;
            }
        }
        Err(_) => return,
    }

    loop {
        tokio::select! {
            update = updates.recv() => {
                let Ok(update) = update else { break; };
                if update.game_id != game_id {
                    continue;
                }
                if let Ok(resp) = build_state_response_for_user(&state, game_id, user.clone(), spectate_intent).await
                    && socket
                        .send(Message::Text(
                            serde_json::to_string(&WsServerMessage::State(Box::new(resp)))
                                .unwrap()
                                .into(),
                        ))
                        .await
                        .is_err()
                {
                    break;
                }
            }
            update = scrap_straighten_updates.recv() => {
                let Ok(update) = update else { break; };
                if update.game_id != game_id {
                    continue;
                }
                if socket
                    .send(Message::Text(
                        serde_json::to_string(&WsServerMessage::ScrapStraighten {
                            game_id: update.game_id,
                            straightened: update.straightened,
                            actor_seat: update.actor_seat,
                        })
                        .unwrap()
                        .into(),
                    ))
                    .await
                    .is_err()
                {
                    break;
                }
            }
            msg = socket.recv() => {
                let Some(msg) = msg else { break; };
                let Ok(msg) = msg else { break; };
                match msg {
                    Message::Text(text) => {
                        match serde_json::from_str::<WsClientMessage>(&text) {
                            Ok(WsClientMessage::Action { expected_version, action }) => {
                                let result = apply_action_internal(&state, game_id, user.clone(), expected_version, action).await;
                                if let Err((code, message)) = result {
                                    let _ = socket
                                        .send(Message::Text(
                                            serde_json::to_string(&WsServerMessage::Error { code, message })
                                                .unwrap()
                                                .into(),
                                        ))
                                        .await;
                                }
                            }
                            Ok(WsClientMessage::ScrapStraighten) => {
                                let result = toggle_scrap_straighten_internal(&state, game_id, user.clone()).await;
                                if let Err((code, message)) = result {
                                    let _ = socket
                                        .send(Message::Text(
                                            serde_json::to_string(&WsServerMessage::Error { code, message })
                                                .unwrap()
                                                .into(),
                                        ))
                                        .await;
                                }
                            }
                            Err(err) => {
                                let _ = socket
                                    .send(Message::Text(
                                        serde_json::to_string(&WsServerMessage::Error {
                                            code: 400,
                                            message: format!("invalid message: {}", err),
                                        })
                                        .unwrap()
                                        .into(),
                                    ))
                                    .await;
                            }
                        }
                    }
                    Message::Close(_) => break,
                    _ => {}
                }
            }
        }
    }
    if spectator_registered {
        set_spectator_disconnected(&state, game_id, user.id).await;
    }
}

async fn handle_lobbies_ws(mut socket: WebSocket, state: AppState, user: AuthUser) {
    let mut updates = state.lobby_updates.subscribe();

    if let Ok(lobby_lists) = lobby_list_for_user(&state, user.id).await {
        let _ = socket
            .send(Message::Text(
                serde_json::to_string(&WsServerMessage::Lobbies {
                    lobbies: lobby_lists.lobbies,
                    spectatable_games: lobby_lists.spectatable_games,
                })
                .unwrap()
                .into(),
            ))
            .await;
    }

    loop {
        tokio::select! {
            update = updates.recv() => {
                let Ok(_) = update else { break; };
                let Ok(lobby_lists) = lobby_list_for_user(&state, user.id).await else { continue; };
                if socket
                    .send(Message::Text(
                        serde_json::to_string(&WsServerMessage::Lobbies {
                            lobbies: lobby_lists.lobbies,
                            spectatable_games: lobby_lists.spectatable_games,
                        })
                            .unwrap()
                            .into(),
                    ))
                    .await
                    .is_err()
                {
                    break;
                }
            }
            msg = socket.recv() => {
                let Some(msg) = msg else { break; };
                let Ok(msg) = msg else { break; };
                if let Message::Close(_) = msg {
                    break;
                }
            }
        }
    }
}

async fn apply_action_internal(
    state: &AppState,
    game_id: i64,
    user: AuthUser,
    expected_version: i64,
    action: Action,
) -> Result<(), (u16, String)> {
    let (tx, rx) = oneshot::channel();
    state
        .store_tx
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

async fn toggle_scrap_straighten_internal(
    state: &AppState,
    game_id: i64,
    user: AuthUser,
) -> Result<(), (u16, String)> {
    let (tx, rx) = oneshot::channel();
    state
        .store_tx
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

async fn validate_viewer(
    state: &AppState,
    game_id: i64,
    user: AuthUser,
    spectate_intent: bool,
) -> Result<(), (u16, String)> {
    let (tx, rx) = oneshot::channel();
    state
        .store_tx
        .send(Command::ValidateViewer {
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

async fn build_state_response_for_user(
    state: &AppState,
    game_id: i64,
    user: AuthUser,
    spectate_intent: bool,
) -> Result<GameStateResponse, StatusCode> {
    let (tx, rx) = oneshot::channel();
    state
        .store_tx
        .send(Command::GetState {
            game_id,
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
    Ok(resp)
}

async fn set_spectator_connected(
    state: &AppState,
    game_id: i64,
    user: AuthUser,
) -> Result<(), StatusCode> {
    let (tx, rx) = oneshot::channel();
    state
        .store_tx
        .send(Command::SpectatorConnected {
            game_id,
            user,
            respond: tx,
        })
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    rx.await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .map_err(|err| err.status_code())
}

async fn set_spectator_disconnected(state: &AppState, game_id: i64, user_id: i64) {
    let _ = state
        .store_tx
        .send(Command::SpectatorDisconnected { game_id, user_id })
        .await;
}

async fn lobby_list_for_user(
    state: &AppState,
    user_id: i64,
) -> Result<LobbyListsResponse, StatusCode> {
    let (tx, rx) = oneshot::channel();
    state
        .store_tx
        .send(Command::GetLobbyListForUser {
            user_id,
            respond: tx,
        })
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    rx.await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn authorize(state: &AppState, headers: &HeaderMap) -> Result<AuthUser, StatusCode> {
    let cookie = headers
        .get(header::COOKIE)
        .ok_or(StatusCode::UNAUTHORIZED)?
        .to_str()
        .map_err(|_| StatusCode::UNAUTHORIZED)?;
    let session_key = extract_session_id(cookie).unwrap_or_else(|| cookie.to_string());

    if let Some(user) = get_cached_user(&state.auth_cache, &session_key).await {
        return Ok(user);
    }

    let url = format!("{}/api/user/status", state.js_base);
    let res = state
        .http
        .get(url)
        .header(header::COOKIE, cookie)
        .send()
        .await
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    if !res.status().is_success() {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let body: AuthStatus = res.json().await.map_err(|_| StatusCode::UNAUTHORIZED)?;
    if !body.authenticated {
        return Err(StatusCode::UNAUTHORIZED);
    }
    let id = body.id.ok_or(StatusCode::UNAUTHORIZED)?;
    let username = body.username.ok_or(StatusCode::UNAUTHORIZED)?;
    let user = AuthUser { id, username };
    set_cached_user(&state.auth_cache, session_key, user.clone()).await;
    Ok(user)
}

#[derive(Deserialize)]
struct AuthStatus {
    authenticated: bool,
    id: Option<i64>,
    username: Option<String>,
}

fn extract_session_id(cookie_header: &str) -> Option<String> {
    cookie_header
        .split(';')
        .map(|part| part.trim())
        .find_map(|part| part.strip_prefix("cuttle.sid=").map(|val| val.to_string()))
}

async fn get_cached_user(
    cache: &Arc<Mutex<HashMap<String, AuthCacheEntry>>>,
    session_key: &str,
) -> Option<AuthUser> {
    let now = Instant::now();
    let mut guard = cache.lock().await;
    guard.retain(|_, entry| entry.expires_at > now);
    guard.get(session_key).map(|entry| entry.user.clone())
}

async fn set_cached_user(
    cache: &Arc<Mutex<HashMap<String, AuthCacheEntry>>>,
    session_key: String,
    user: AuthUser,
) {
    let mut guard = cache.lock().await;
    guard.insert(
        session_key,
        AuthCacheEntry {
            user,
            expires_at: Instant::now() + AUTH_CACHE_TTL,
        },
    );
}

#[derive(Clone, Debug)]
struct SeatEntry {
    seat: Seat,
    user_id: i64,
    username: String,
    ready: bool,
}

#[derive(Clone, Debug)]
struct GameEntry {
    id: i64,
    name: String,
    status: i16,
    is_rematch_lobby: bool,
    rematch_from_game_id: Option<i64>,
    series_anchor_game_id: i64,
    series_player_order: Vec<i64>,
    seats: Vec<SeatEntry>,
    tokenlog_full: String,
    last_event: Option<LastEventView>,
    scrap_straightened: bool,
    started_at: Option<DateTime<Utc>>,
    finished_at: Option<DateTime<Utc>>,
    active_spectators: HashMap<i64, (String, usize)>,
    version: i64,
    engine: CutthroatState,
}

#[derive(Debug)]
enum StoreError {
    NotFound,
    Forbidden,
    Conflict,
    BadRequest,
}

impl StoreError {
    fn status_code(&self) -> StatusCode {
        match self {
            StoreError::NotFound => StatusCode::NOT_FOUND,
            StoreError::Forbidden => StatusCode::FORBIDDEN,
            StoreError::Conflict => StatusCode::CONFLICT,
            StoreError::BadRequest => StatusCode::BAD_REQUEST,
        }
    }

    fn code(&self) -> u16 {
        self.status_code().as_u16()
    }

    fn message(&self) -> String {
        match self {
            StoreError::NotFound => "not found".to_string(),
            StoreError::Forbidden => "forbidden".to_string(),
            StoreError::Conflict => "conflict".to_string(),
            StoreError::BadRequest => "bad request".to_string(),
        }
    }
}

struct Store {
    next_id: i64,
    games: HashMap<i64, GameEntry>,
    rematches: HashMap<i64, i64>,
    updates: broadcast::Sender<GameUpdate>,
    lobby_updates: broadcast::Sender<LobbyListUpdate>,
    scrap_straighten_updates: broadcast::Sender<ScrapStraightenUpdate>,
}

impl Store {
    fn active_spectator_usernames(game: &GameEntry) -> Vec<String> {
        let mut names: Vec<String> = game
            .active_spectators
            .values()
            .filter(|(_, count)| *count > 0)
            .map(|(username, _)| username.clone())
            .collect();
        names.sort();
        names
    }

    fn new(
        updates: broadcast::Sender<GameUpdate>,
        lobby_updates: broadcast::Sender<LobbyListUpdate>,
        scrap_straighten_updates: broadcast::Sender<ScrapStraightenUpdate>,
    ) -> Self {
        Self {
            next_id: 1,
            games: HashMap::new(),
            rematches: HashMap::new(),
            updates,
            lobby_updates,
            scrap_straighten_updates,
        }
    }

    fn lobby_list_for_user(&self, user_id: Option<i64>) -> LobbyListsResponse {
        let mut lobbies: Vec<LobbySummary> = self
            .games
            .values()
            .filter(|game| game.status == STATUS_LOBBY)
            .filter(|game| {
                if !game.is_rematch_lobby {
                    return true;
                }
                match user_id {
                    Some(uid) => game.seats.iter().any(|seat| seat.user_id == uid),
                    None => false,
                }
            })
            .map(|game| LobbySummary {
                id: game.id,
                name: game.name.clone(),
                seat_count: game.seats.len(),
                ready_count: game.seats.iter().filter(|seat| seat.ready).count(),
                status: game.status,
            })
            .collect();
        lobbies.sort_by_key(|lobby| lobby.id);

        let mut spectatable_games: Vec<SpectatableGameSummary> = self
            .games
            .values()
            .filter(|game| game.status == STATUS_STARTED)
            .map(|game| {
                SpectatableGameSummary {
                    id: game.id,
                    name: game.name.clone(),
                    seat_count: game.seats.len(),
                    status: game.status,
                    spectating_usernames: Self::active_spectator_usernames(game),
                }
            })
            .collect();
        spectatable_games.sort_by_key(|game| game.id);

        LobbyListsResponse {
            lobbies,
            spectatable_games,
        }
    }

    fn broadcast_lobbies(&self) {
        let _ = self.lobby_updates.send(LobbyListUpdate { _changed: true });
    }

    fn broadcast_game(&self, game_id: i64) {
        let _ = self.updates.send(GameUpdate { game_id });
    }

    fn create_game(&mut self, user: AuthUser) -> i64 {
        let id = self.next_id;
        self.next_id += 1;

        let mut deck = cutthroat_engine::full_deck_with_jokers();
        deck.shuffle(&mut rand::thread_rng());
        let header = encode_header(0, &deck);
        let engine = CutthroatState::new_with_deck(0, deck);

        let seat = SeatEntry {
            seat: 0,
            user_id: user.id,
            username: user.username,
            ready: false,
        };

        let game = GameEntry {
            id,
            name: String::new(),
            status: STATUS_LOBBY,
            is_rematch_lobby: false,
            rematch_from_game_id: None,
            series_anchor_game_id: id,
            series_player_order: Vec::new(),
            seats: vec![seat],
            tokenlog_full: header,
            last_event: None,
            scrap_straightened: false,
            started_at: None,
            finished_at: None,
            active_spectators: HashMap::new(),
            version: 0,
            engine,
        };

        let mut game = game;
        game.name = normal_lobby_name(&game.seats);
        self.games.insert(id, game);
        self.broadcast_lobbies();
        id
    }

    fn rematch_game(&mut self, game_id: i64, user: AuthUser) -> Result<i64, StoreError> {
        let prior_game = self.games.get(&game_id).ok_or(StoreError::NotFound)?;
        if prior_game.status != STATUS_FINISHED {
            return Err(StoreError::Conflict);
        }
        if !prior_game.seats.iter().any(|seat| seat.user_id == user.id) {
            return Err(StoreError::Forbidden);
        }

        if let Some(existing_id) = self.rematches.get(&game_id).copied()
            && self
                .games
                .get(&existing_id)
                .map(|game| game.status == STATUS_LOBBY)
                .unwrap_or(false)
        {
            return Ok(existing_id);
        }

        let id = self.next_id;
        self.next_id += 1;

        let mut deck = cutthroat_engine::full_deck_with_jokers();
        deck.shuffle(&mut rand::thread_rng());
        let header = encode_header(0, &deck);
        let engine = CutthroatState::new_with_deck(0, deck);

        let mut seats = prior_game.seats.clone();
        seats.sort_by_key(|seat| seat.seat);
        for seat in &mut seats {
            seat.ready = false;
        }

        let series_order = if prior_game.series_player_order.is_empty() {
            seats.iter().map(|seat| seat.user_id).collect::<Vec<i64>>()
        } else {
            prior_game.series_player_order.clone()
        };

        let rematch_name = self.rematch_series_name(prior_game, &series_order);

        let game = GameEntry {
            id,
            name: rematch_name,
            status: STATUS_LOBBY,
            is_rematch_lobby: true,
            rematch_from_game_id: Some(prior_game.id),
            series_anchor_game_id: prior_game.series_anchor_game_id,
            series_player_order: series_order,
            seats,
            tokenlog_full: header,
            last_event: None,
            scrap_straightened: false,
            started_at: None,
            finished_at: None,
            active_spectators: HashMap::new(),
            version: 0,
            engine,
        };

        self.games.insert(id, game);
        self.rematches.insert(game_id, id);
        self.broadcast_lobbies();
        self.broadcast_game(id);
        Ok(id)
    }

    fn join_game(&mut self, game_id: i64, user: AuthUser) -> Result<Seat, StoreError> {
        let game = self.games.get_mut(&game_id).ok_or(StoreError::NotFound)?;
        if game.status != STATUS_LOBBY {
            return Err(StoreError::Conflict);
        }

        if let Some(existing) = game.seats.iter().find(|seat| seat.user_id == user.id) {
            return Ok(existing.seat);
        }

        if game.seats.len() >= 3 {
            return Err(StoreError::Conflict);
        }

        let mut occupied = [false; 3];
        for seat in &game.seats {
            occupied[seat.seat as usize] = true;
        }
        let seat_index = occupied
            .iter()
            .position(|v| !*v)
            .ok_or(StoreError::Conflict)?;

        game.seats.push(SeatEntry {
            seat: seat_index as Seat,
            user_id: user.id,
            username: user.username,
            ready: false,
        });
        game.name = normal_lobby_name(&game.seats);

        self.broadcast_lobbies();
        self.broadcast_game(game_id);
        Ok(seat_index as Seat)
    }

    fn leave_game(&mut self, game_id: i64, user: AuthUser) -> Result<(), StoreError> {
        let mut should_remove_game = false;
        {
            let game = self.games.get_mut(&game_id).ok_or(StoreError::NotFound)?;
            if game.status != STATUS_LOBBY {
                return Err(StoreError::Conflict);
            }
            let idx = game
                .seats
                .iter()
                .position(|seat| seat.user_id == user.id)
                .ok_or(StoreError::Forbidden)?;
            game.seats.remove(idx);
            if game.seats.is_empty() {
                should_remove_game = true;
            } else if !game.is_rematch_lobby {
                game.name = normal_lobby_name(&game.seats);
            }
        }

        if should_remove_game {
            self.games.remove(&game_id);
            self.rematches
                .retain(|_, rematch_id| *rematch_id != game_id);
        } else {
            self.broadcast_game(game_id);
        }
        self.broadcast_lobbies();
        Ok(())
    }

    fn set_ready(&mut self, game_id: i64, user: AuthUser, ready: bool) -> Result<(), StoreError> {
        let game = self.games.get_mut(&game_id).ok_or(StoreError::NotFound)?;
        if game.status != STATUS_LOBBY {
            return Err(StoreError::Conflict);
        }
        let seat = game
            .seats
            .iter_mut()
            .find(|seat| seat.user_id == user.id)
            .ok_or(StoreError::Forbidden)?;
        seat.ready = ready;

        if game.status == STATUS_LOBBY
            && game.seats.len() == 3
            && game.seats.iter().all(|seat| seat.ready)
        {
            game.status = STATUS_STARTED;
            if game.started_at.is_none() {
                game.started_at = Some(Utc::now());
            }
        }

        self.broadcast_lobbies();
        self.broadcast_game(game_id);
        Ok(())
    }

    fn start_game(&mut self, game_id: i64) -> Result<(), StoreError> {
        let game = self.games.get_mut(&game_id).ok_or(StoreError::NotFound)?;
        if game.status != STATUS_LOBBY {
            return Err(StoreError::Conflict);
        }
        if game.seats.len() != 3 || !game.seats.iter().all(|seat| seat.ready) {
            return Err(StoreError::Conflict);
        }
        game.status = STATUS_STARTED;
        if game.started_at.is_none() {
            game.started_at = Some(Utc::now());
        }
        self.broadcast_lobbies();
        self.broadcast_game(game_id);
        Ok(())
    }

    fn validate_viewer(
        &self,
        game_id: i64,
        user: &AuthUser,
        spectate_intent: bool,
    ) -> Result<(), StoreError> {
        let game = self.games.get(&game_id).ok_or(StoreError::NotFound)?;
        let viewer_is_seated = game.seats.iter().any(|seat| seat.user_id == user.id);
        if spectate_intent {
            if viewer_is_seated {
                return Err(StoreError::Conflict);
            }
            if game.status != STATUS_STARTED && game.status != STATUS_FINISHED {
                return Err(StoreError::Conflict);
            }
            return Ok(());
        }

        if viewer_is_seated {
            return Ok(());
        }
        if game.status == STATUS_LOBBY {
            return Err(StoreError::Conflict);
        }
        Ok(())
    }

    fn build_spectator_state_response(&self, game: &GameEntry) -> GameStateResponse {
        let lobby = LobbyView {
            seats: game
                .seats
                .iter()
                .map(|seat| LobbySeatView {
                    seat: seat.seat,
                    user_id: seat.user_id,
                    username: seat.username.clone(),
                    ready: seat.ready,
                })
                .collect(),
        };
        let spectator_view = build_spectator_view(game);
        let action_seat = match &game.engine.phase {
            Phase::ResolvingSeven { seat, .. } => *seat,
            _ => game.engine.turn,
        };
        let mut legal_actions = if game.status == STATUS_STARTED {
            game.engine.legal_actions(action_seat)
        } else {
            Vec::new()
        };
        legal_actions.sort_by_key(format_action);
        let log_tail = build_history_log_for_viewer(game, 0);
        let tokenlog = redact_tokenlog_for_client(&game.tokenlog_full);
        GameStateResponse {
            version: game.version,
            seat: 0,
            status: game.status,
            player_view: spectator_view.clone(),
            spectator_view,
            legal_actions,
            lobby,
            log_tail,
            tokenlog,
            is_spectator: true,
            spectating_usernames: Self::active_spectator_usernames(game),
        }
    }

    fn build_state_response_for_user(
        &self,
        game_id: i64,
        user: &AuthUser,
        spectate_intent: bool,
    ) -> Result<GameStateResponse, StoreError> {
        self.validate_viewer(game_id, user, spectate_intent)?;
        let game = self.games.get(&game_id).ok_or(StoreError::NotFound)?;
        let maybe_seat = game
            .seats
            .iter()
            .find(|seat| seat.user_id == user.id)
            .map(|seat| seat.seat);
        if spectate_intent || maybe_seat.is_none() {
            return Ok(self.build_spectator_state_response(game));
        }
        self.build_state_response(game_id, maybe_seat.unwrap_or(0))
    }

    fn build_state_response(
        &self,
        game_id: i64,
        seat: Seat,
    ) -> Result<GameStateResponse, StoreError> {
        let game = self.games.get(&game_id).ok_or(StoreError::NotFound)?;
        let lobby = LobbyView {
            seats: game
                .seats
                .iter()
                .map(|seat| LobbySeatView {
                    seat: seat.seat,
                    user_id: seat.user_id,
                    username: seat.username.clone(),
                    ready: seat.ready,
                })
                .collect(),
        };
        let mut legal_actions = if game.status == STATUS_STARTED {
            game.engine.legal_actions(seat)
        } else {
            Vec::new()
        };
        legal_actions.sort_by_key(format_action);
        let mut player_view = game.engine.public_view(seat);
        player_view.last_event = game.last_event.clone();
        let spectator_view = build_spectator_view(game);
        let log_tail = build_history_log_for_viewer(game, seat);
        let tokenlog = redact_tokenlog_for_client(&game.tokenlog_full);
        Ok(GameStateResponse {
            version: game.version,
            seat,
            status: game.status,
            player_view,
            spectator_view,
            legal_actions,
            lobby,
            log_tail,
            tokenlog,
            is_spectator: false,
            spectating_usernames: Self::active_spectator_usernames(game),
        })
    }

    fn spectator_connected(&mut self, game_id: i64, user: AuthUser) -> Result<(), StoreError> {
        let game = self.games.get_mut(&game_id).ok_or(StoreError::NotFound)?;
        if game.seats.iter().any(|seat| seat.user_id == user.id) {
            return Err(StoreError::Conflict);
        }
        let entry = game
            .active_spectators
            .entry(user.id)
            .or_insert((user.username, 0));
        entry.1 += 1;
        self.broadcast_game(game_id);
        self.broadcast_lobbies();
        Ok(())
    }

    fn spectator_disconnected(&mut self, game_id: i64, user_id: i64) {
        let Some(game) = self.games.get_mut(&game_id) else {
            return;
        };
        let mut changed = false;
        if let Some((_, count)) = game.active_spectators.get_mut(&user_id) {
            if *count > 1 {
                *count -= 1;
                changed = true;
            } else {
                game.active_spectators.remove(&user_id);
                changed = true;
            }
        }
        if changed {
            self.broadcast_game(game_id);
            self.broadcast_lobbies();
        }
    }

    fn apply_action(
        &mut self,
        game_id: i64,
        user: AuthUser,
        expected_version: i64,
        action: Action,
    ) -> Result<(GameStateResponse, Option<CompletedGameRecord>), StoreError> {
        let game = self.games.get_mut(&game_id).ok_or(StoreError::NotFound)?;
        if game.status != STATUS_STARTED {
            return Err(StoreError::Conflict);
        }
        let seat = game
            .seats
            .iter()
            .find(|seat| seat.user_id == user.id)
            .map(|seat| seat.seat)
            .ok_or(StoreError::Forbidden)?;
        if game.version != expected_version {
            return Err(StoreError::Conflict);
        }

        let scrap_len_before = game.engine.scrap.len();
        let phase_before = game.engine.phase.clone();
        game.engine
            .apply(seat, action.clone())
            .map_err(|_| StoreError::BadRequest)?;
        append_action(&mut game.tokenlog_full, seat, &action)
            .map_err(|_| StoreError::BadRequest)?;
        game.last_event = Some(build_last_event(seat, &action, &phase_before));
        game.version += 1;
        let mut completed_record = None;
        if game.engine.winner.is_some() && game.status != STATUS_FINISHED {
            game.status = STATUS_FINISHED;
            let finished_at = Utc::now();
            game.finished_at = Some(finished_at);
            completed_record = Self::build_completed_record(game, finished_at);
        }
        if game.engine.scrap.len() > scrap_len_before && game.scrap_straightened {
            game.scrap_straightened = false;
            let _ = self.scrap_straighten_updates.send(ScrapStraightenUpdate {
                game_id,
                straightened: false,
                actor_seat: seat,
            });
        }

        self.broadcast_game(game_id);
        let state = self.build_state_response(game_id, seat)?;
        Ok((state, completed_record))
    }

    fn usernames_by_seat(game: &GameEntry) -> Option<[String; 3]> {
        usernames_from_seats(&game.seats)
    }

    fn build_completed_record(
        game: &GameEntry,
        finished_at: DateTime<Utc>,
    ) -> Option<CompletedGameRecord> {
        let started_at = game.started_at?;
        let [p0_username, p1_username, p2_username] = Self::usernames_by_seat(game)?;
        Some(CompletedGameRecord {
            rust_game_id: game.id,
            tokenlog: game.tokenlog_full.clone(),
            p0_username,
            p1_username,
            p2_username,
            started_at,
            finished_at,
        })
    }

    fn toggle_scrap_straighten(
        &mut self,
        game_id: i64,
        user: AuthUser,
    ) -> Result<ScrapStraightenUpdate, StoreError> {
        let game = self.games.get_mut(&game_id).ok_or(StoreError::NotFound)?;
        let seat = game
            .seats
            .iter()
            .find(|seat| seat.user_id == user.id)
            .map(|seat| seat.seat)
            .ok_or(StoreError::Forbidden)?;

        game.scrap_straightened = !game.scrap_straightened;
        let update = ScrapStraightenUpdate {
            game_id,
            straightened: game.scrap_straightened,
            actor_seat: seat,
        };
        let _ = self.scrap_straighten_updates.send(update.clone());
        Ok(update)
    }

    fn username_for_user_id(&self, user_id: i64) -> String {
        self.games
            .values()
            .flat_map(|game| game.seats.iter())
            .find(|seat| seat.user_id == user_id)
            .map(|seat| seat.username.clone())
            .unwrap_or_else(|| format!("User {}", user_id))
    }

    fn winner_user_id(game: &GameEntry) -> Option<i64> {
        let winner_seat = match game.engine.winner.as_ref()? {
            Winner::Seat(seat) => *seat,
            Winner::Draw => return None,
        };
        game.seats
            .iter()
            .find(|seat| seat.seat == winner_seat)
            .map(|seat| seat.user_id)
    }

    fn rematch_series_name(&self, prior_game: &GameEntry, series_order: &[i64]) -> String {
        let mut chain = Vec::new();
        let mut cursor = Some(prior_game.id);
        while let Some(game_id) = cursor {
            let Some(game) = self.games.get(&game_id) else {
                break;
            };
            chain.push(game);
            cursor = game.rematch_from_game_id;
        }
        chain.reverse();

        let mut wins = [0usize; 3];
        let mut stalemates = 0usize;
        for game in chain {
            if game.status != STATUS_FINISHED {
                continue;
            }
            match Self::winner_user_id(game) {
                Some(user_id) => {
                    if let Some(idx) = series_order.iter().position(|id| *id == user_id)
                        && idx < 3
                    {
                        wins[idx] += 1;
                    }
                }
                None => {
                    stalemates += 1;
                }
            }
        }

        let names: Vec<String> = series_order
            .iter()
            .take(3)
            .map(|id| self.username_for_user_id(*id))
            .collect();
        let n0 = names.first().cloned().unwrap_or_else(|| "P1".to_string());
        let n1 = names.get(1).cloned().unwrap_or_else(|| "P2".to_string());
        let n2 = names.get(2).cloned().unwrap_or_else(|| "P3".to_string());
        format!(
            "{} VS {} VS {} {}-{}-{}-{}",
            n0, n1, n2, wins[0], wins[1], wins[2], stalemates
        )
    }
}

fn usernames_from_seats(seats: &[SeatEntry]) -> Option<[String; 3]> {
    let mut usernames: [Option<String>; 3] = [None, None, None];
    for seat in seats {
        let idx = seat.seat as usize;
        if idx < 3 {
            usernames[idx] = Some(seat.username.clone());
        }
    }
    Some([
        usernames[0].clone()?,
        usernames[1].clone()?,
        usernames[2].clone()?,
    ])
}

fn normal_lobby_name(seats: &[SeatEntry]) -> String {
    let mut by_seat = [
        String::from("Open"),
        String::from("Open"),
        String::from("Open"),
    ];
    for seat in seats {
        let idx = seat.seat as usize;
        if idx < 3 {
            by_seat[idx] = seat.username.clone();
        }
    }
    format!("{} VS {} VS {}", by_seat[0], by_seat[1], by_seat[2])
}

fn build_spectator_view(game: &GameEntry) -> PublicView {
    let viewer = match &game.engine.phase {
        Phase::ResolvingSeven { seat, .. } => *seat,
        _ => game.engine.turn,
    };
    let mut view = game.engine.public_view(viewer);
    for (idx, player) in game.engine.players.iter().enumerate() {
        if let Some(player_view) = view.players.get_mut(idx) {
            player_view.hand = player
                .hand
                .iter()
                .map(|card| PublicCard::Known(card.to_token()))
                .collect();
            player_view.frozen = player
                .frozen
                .iter()
                .map(|card| card.card.to_token())
                .collect();
        }
    }
    // Stub spectator behavior: reveal all public/private board state but not deck info.
    view.deck_count = 0;
    view.last_event = game.last_event.clone();
    view
}

fn format_action(action: &Action) -> String {
    match action {
        Action::Draw => "draw".to_string(),
        Action::Pass => "pass".to_string(),
        Action::PlayPoints { .. } => "points".to_string(),
        Action::Scuttle { .. } => "scuttle".to_string(),
        Action::PlayRoyal { .. } => "royal".to_string(),
        Action::PlayJack { .. } => "jack".to_string(),
        Action::PlayJoker { .. } => "joker".to_string(),
        Action::PlayOneOff { .. } => "oneoff".to_string(),
        Action::CounterTwo { .. } => "counter_two".to_string(),
        Action::CounterPass => "counter_pass".to_string(),
        Action::ResolveThreePick { .. } => "resolve_three".to_string(),
        Action::ResolveFourDiscard { .. } => "resolve_four".to_string(),
        Action::ResolveFiveDiscard { .. } => "resolve_five".to_string(),
        Action::ResolveSevenChoose { .. } => "resolve_seven".to_string(),
    }
}

fn oneoff_target_fields(target: &OneOffTarget) -> (Option<String>, Option<Seat>, Option<String>) {
    match target {
        OneOffTarget::None => (None, None, None),
        OneOffTarget::Player { seat } => (None, Some(*seat), Some("player".to_string())),
        OneOffTarget::Point { base } => (Some(base.to_token()), None, Some("point".to_string())),
        OneOffTarget::Royal { card } => (Some(card.to_token()), None, Some("royal".to_string())),
        OneOffTarget::Jack { card } => (Some(card.to_token()), None, Some("jack".to_string())),
        OneOffTarget::Joker { card } => (Some(card.to_token()), None, Some("joker".to_string())),
    }
}

fn build_last_event(actor: Seat, action: &Action, phase_before: &Phase) -> LastEventView {
    let action_kind = format_action(action);
    let mut change = "main".to_string();
    let mut source_token: Option<String> = None;
    let source_zone: Option<String>;
    let mut target_token: Option<String> = None;
    let mut target_seat: Option<Seat> = None;
    let mut target_type: Option<String> = None;
    let mut oneoff_rank: Option<u8> = None;

    match action {
        Action::Draw => {
            source_zone = Some("deck".to_string());
        }
        Action::Pass => {
            source_zone = Some("deck".to_string());
        }
        Action::PlayPoints { card } => {
            source_zone = Some("hand".to_string());
            source_token = Some(card.to_token());
        }
        Action::Scuttle {
            card,
            target_point_base,
        } => {
            change = "scuttle".to_string();
            source_zone = Some("hand".to_string());
            source_token = Some(card.to_token());
            target_token = Some(target_point_base.to_token());
            target_type = Some("point".to_string());
        }
        Action::PlayRoyal { card } => {
            source_zone = Some("hand".to_string());
            source_token = Some(card.to_token());
        }
        Action::PlayJack {
            jack,
            target_point_base,
        } => {
            change = "jack".to_string();
            source_zone = Some("hand".to_string());
            source_token = Some(jack.to_token());
            target_token = Some(target_point_base.to_token());
            target_type = Some("point".to_string());
        }
        Action::PlayJoker {
            joker,
            target_royal_card,
        } => {
            change = "joker".to_string();
            source_zone = Some("hand".to_string());
            source_token = Some(joker.to_token());
            target_token = Some(target_royal_card.to_token());
            target_type = Some("royal".to_string());
        }
        Action::PlayOneOff { card, target } => {
            change = "counter".to_string();
            source_zone = Some("hand".to_string());
            source_token = Some(card.to_token());
            oneoff_rank = card.rank_value();
            let (target_token_val, target_seat_val, target_type_val) = oneoff_target_fields(target);
            target_token = target_token_val;
            target_seat = target_seat_val;
            target_type = target_type_val;
        }
        Action::CounterTwo { two_card } => {
            change = "counter".to_string();
            source_zone = Some("hand".to_string());
            source_token = Some(two_card.to_token());
        }
        Action::CounterPass => {
            change = "counter".to_string();
            source_zone = Some("counter".to_string());
            source_token = Some("pass".to_string());
        }
        Action::ResolveThreePick { card_from_scrap } => {
            change = "resolve".to_string();
            source_zone = Some("scrap".to_string());
            source_token = Some(card_from_scrap.to_token());
        }
        Action::ResolveFourDiscard { card } => {
            change = "resolve".to_string();
            source_zone = Some("hand".to_string());
            source_token = Some(card.to_token());
        }
        Action::ResolveFiveDiscard { card } => {
            change = "resolve".to_string();
            source_zone = Some("hand".to_string());
            source_token = Some(card.to_token());
        }
        Action::ResolveSevenChoose { source_index, play } => {
            source_zone = Some("reveal".to_string());
            source_token = Some(format!("reveal:{}", source_index));
            match play {
                SevenPlay::Points => {
                    change = "main".to_string();
                }
                SevenPlay::Scuttle { target } => {
                    change = "scuttle".to_string();
                    target_token = Some(target.to_token());
                    target_type = Some("point".to_string());
                }
                SevenPlay::Royal => {
                    change = "main".to_string();
                }
                SevenPlay::Jack { target } => {
                    change = "sevenJack".to_string();
                    target_token = Some(target.to_token());
                    target_type = Some("point".to_string());
                }
                SevenPlay::Joker { target } => {
                    change = "joker".to_string();
                    target_token = Some(target.to_token());
                    target_type = Some("royal".to_string());
                }
                SevenPlay::OneOff { target } => {
                    change = "resolve".to_string();
                    let (target_token_val, target_seat_val, target_type_val) =
                        oneoff_target_fields(target);
                    target_token = target_token_val;
                    target_seat = target_seat_val;
                    target_type = target_type_val;
                }
                SevenPlay::Discard => {
                    change = "resolve".to_string();
                }
            }
        }
    }

    if let Phase::Countering(counter) = phase_before
        && matches!(action, Action::CounterPass | Action::CounterTwo { .. })
    {
        if let Action::PlayOneOff { target, card } = &counter.oneoff {
            oneoff_rank = oneoff_rank.or(card.rank_value());
            if target_type.is_none() {
                let (target_token_val, target_seat_val, target_type_val) =
                    oneoff_target_fields(target);
                target_token = target_token.or(target_token_val);
                target_seat = target_seat.or(target_seat_val);
                target_type = target_type.or(target_type_val);
            }
        }

        if matches!(action, Action::CounterPass) {
            let next_after_pass = (counter.next_seat + 1) % 3;
            if next_after_pass == counter.rotation_anchor {
                change = "resolve".to_string();
            }
        }
    }

    LastEventView {
        actor,
        action_kind,
        change,
        source_token,
        source_zone,
        target_token,
        target_seat,
        target_type,
        oneoff_rank,
    }
}

fn redact_tokenlog_for_client(full_tokenlog: &str) -> String {
    let Ok(parsed) = parse_tokenlog(full_tokenlog) else {
        return encode_header(0, &[]);
    };
    let mut redacted = encode_header(parsed.dealer, &[]);
    for (seat, action) in parsed.actions {
        if append_action(&mut redacted, seat, &action).is_err() {
            break;
        }
    }
    redacted
}

fn build_history_log_for_viewer(game: &GameEntry, viewer: Seat) -> Vec<String> {
    let Ok(parsed) = parse_tokenlog(&game.tokenlog_full) else {
        return Vec::new();
    };
    let mut state = CutthroatState::new_with_deck(parsed.dealer, parsed.deck);
    let seat_names = seat_name_map(&game.seats);
    let mut lines = Vec::new();

    for (actor_seat, action) in parsed.actions {
        if state.apply(actor_seat, action.clone()).is_err() {
            break;
        }
        let view = state.public_view(viewer);
        let visible_tokens = collect_visible_tokens(&view);
        lines.push(format_history_line(
            &action,
            actor_seat,
            &seat_names,
            &visible_tokens,
        ));
    }

    if lines.len() > LOG_TAIL_LIMIT {
        lines.drain(0..(lines.len() - LOG_TAIL_LIMIT));
    }
    lines
}

fn seat_name_map(seats: &[SeatEntry]) -> HashMap<Seat, String> {
    seats
        .iter()
        .map(|seat| (seat.seat, seat.username.clone()))
        .collect()
}

fn seat_name(seat: Seat, seat_names: &HashMap<Seat, String>) -> String {
    seat_names
        .get(&seat)
        .cloned()
        .unwrap_or_else(|| format!("Player {}", seat + 1))
}

fn collect_visible_tokens(view: &PublicView) -> HashSet<String> {
    let mut visible = HashSet::new();

    for token in &view.scrap {
        visible.insert(token.clone());
    }

    for player in &view.players {
        for hand_card in &player.hand {
            if let PublicCard::Known(token) = hand_card {
                visible.insert(token.clone());
            }
        }
        for point in &player.points {
            visible.insert(point.base.clone());
            for jack in &point.jacks {
                visible.insert(jack.clone());
            }
        }
        for royal in &player.royals {
            visible.insert(royal.base.clone());
            for joker in &royal.jokers {
                visible.insert(joker.clone());
            }
        }
        for frozen in &player.frozen {
            visible.insert(frozen.clone());
        }
    }

    match &view.phase {
        PhaseView::Countering { oneoff, twos, .. } => {
            add_action_tokens(oneoff, &mut visible);
            for two in twos {
                visible.insert(two.card.clone());
            }
        }
        PhaseView::ResolvingSeven { revealed_cards, .. } => {
            for token in revealed_cards {
                visible.insert(token.clone());
            }
        }
        _ => {}
    }

    visible
}

fn add_action_tokens(action: &Action, visible: &mut HashSet<String>) {
    match action {
        Action::PlayPoints { card } => {
            visible.insert(card.to_token());
        }
        Action::Scuttle {
            card,
            target_point_base,
        } => {
            visible.insert(card.to_token());
            visible.insert(target_point_base.to_token());
        }
        Action::PlayRoyal { card } => {
            visible.insert(card.to_token());
        }
        Action::PlayJack {
            jack,
            target_point_base,
        } => {
            visible.insert(jack.to_token());
            visible.insert(target_point_base.to_token());
        }
        Action::PlayJoker {
            joker,
            target_royal_card,
        } => {
            visible.insert(joker.to_token());
            visible.insert(target_royal_card.to_token());
        }
        Action::PlayOneOff { card, target } => {
            visible.insert(card.to_token());
            add_oneoff_target_tokens(target, visible);
        }
        Action::CounterTwo { two_card } => {
            visible.insert(two_card.to_token());
        }
        Action::ResolveThreePick { card_from_scrap } => {
            visible.insert(card_from_scrap.to_token());
        }
        Action::ResolveFourDiscard { card } => {
            visible.insert(card.to_token());
        }
        Action::ResolveFiveDiscard { card } => {
            visible.insert(card.to_token());
        }
        Action::ResolveSevenChoose { play, .. } => add_seven_play_tokens(play, visible),
        Action::Draw | Action::Pass | Action::CounterPass => {}
    }
}

fn add_oneoff_target_tokens(target: &OneOffTarget, visible: &mut HashSet<String>) {
    match target {
        OneOffTarget::Point { base } => {
            visible.insert(base.to_token());
        }
        OneOffTarget::Royal { card } => {
            visible.insert(card.to_token());
        }
        OneOffTarget::Jack { card } => {
            visible.insert(card.to_token());
        }
        OneOffTarget::Joker { card } => {
            visible.insert(card.to_token());
        }
        OneOffTarget::None | OneOffTarget::Player { .. } => {}
    }
}

fn add_seven_play_tokens(play: &SevenPlay, visible: &mut HashSet<String>) {
    match play {
        SevenPlay::Scuttle { target } => {
            visible.insert(target.to_token());
        }
        SevenPlay::Jack { target } => {
            visible.insert(target.to_token());
        }
        SevenPlay::Joker { target } => {
            visible.insert(target.to_token());
        }
        SevenPlay::OneOff { target } => {
            add_oneoff_target_tokens(target, visible);
        }
        SevenPlay::Points | SevenPlay::Royal | SevenPlay::Discard => {}
    }
}

fn format_history_line(
    action: &Action,
    actor_seat: Seat,
    seat_names: &HashMap<Seat, String>,
    visible_tokens: &HashSet<String>,
) -> String {
    let actor = seat_name(actor_seat, seat_names);
    match action {
        Action::Draw => format!("{} drew a card.", actor),
        Action::Pass => format!("{} passed.", actor),
        Action::PlayPoints { card } => {
            format!(
                "{} played the {} for points.",
                actor,
                card_name_for_history(*card, visible_tokens)
            )
        }
        Action::Scuttle {
            card,
            target_point_base,
        } => format!(
            "{} scuttled the {} with the {}.",
            actor,
            card_name_for_history(*target_point_base, visible_tokens),
            card_name_for_history(*card, visible_tokens)
        ),
        Action::PlayRoyal { card } => format!(
            "{} played the {} as a royal.",
            actor,
            card_name_for_history(*card, visible_tokens)
        ),
        Action::PlayJack {
            jack,
            target_point_base,
        } => format!(
            "{} stole the {} with the {}.",
            actor,
            card_name_for_history(*target_point_base, visible_tokens),
            card_name_for_history(*jack, visible_tokens)
        ),
        Action::PlayJoker {
            joker,
            target_royal_card,
        } => format!(
            "{} played the {} on the {}.",
            actor,
            card_name_for_history(*joker, visible_tokens),
            card_name_for_history(*target_royal_card, visible_tokens)
        ),
        Action::PlayOneOff { card, target } => format!(
            "{} played the {} as a one-off{}.",
            actor,
            card_name_for_history(*card, visible_tokens),
            oneoff_target_text(target, seat_names, visible_tokens)
        ),
        Action::CounterTwo { two_card } => format!(
            "{} played the {} to counter.",
            actor,
            card_name_for_history(*two_card, visible_tokens)
        ),
        Action::CounterPass => format!("{} passed counter.", actor),
        Action::ResolveThreePick { card_from_scrap } => format!(
            "{} took the {} from scrap.",
            actor,
            card_name_for_history(*card_from_scrap, visible_tokens)
        ),
        Action::ResolveFourDiscard { card } => format!(
            "{} discarded the {}.",
            actor,
            card_name_for_history(*card, visible_tokens)
        ),
        Action::ResolveFiveDiscard { card } => format!(
            "{} discarded the {}.",
            actor,
            card_name_for_history(*card, visible_tokens)
        ),
        Action::ResolveSevenChoose { source_index, play } => format!(
            "{} resolved seven from revealed card {}{}.",
            actor,
            source_index + 1,
            seven_play_text(play, seat_names, visible_tokens)
        ),
    }
}

fn oneoff_target_text(
    target: &OneOffTarget,
    seat_names: &HashMap<Seat, String>,
    visible_tokens: &HashSet<String>,
) -> String {
    match target {
        OneOffTarget::None => String::new(),
        OneOffTarget::Player { seat } => format!(", targeting {}", seat_name(*seat, seat_names)),
        OneOffTarget::Point { base } => format!(
            ", targeting the {}",
            card_name_for_history(*base, visible_tokens)
        ),
        OneOffTarget::Royal { card } => format!(
            ", targeting the {}",
            card_name_for_history(*card, visible_tokens)
        ),
        OneOffTarget::Jack { card } => format!(
            ", targeting the {}",
            card_name_for_history(*card, visible_tokens)
        ),
        OneOffTarget::Joker { card } => format!(
            ", targeting the {}",
            card_name_for_history(*card, visible_tokens)
        ),
    }
}

fn seven_play_text(
    play: &SevenPlay,
    seat_names: &HashMap<Seat, String>,
    visible_tokens: &HashSet<String>,
) -> String {
    match play {
        SevenPlay::Points => " as points".to_string(),
        SevenPlay::Scuttle { target } => format!(
            " as scuttle targeting {}",
            card_name_for_history(*target, visible_tokens)
        ),
        SevenPlay::Royal => " as a royal".to_string(),
        SevenPlay::Jack { target } => format!(
            " as a jack targeting {}",
            card_name_for_history(*target, visible_tokens)
        ),
        SevenPlay::Joker { target } => format!(
            " as a joker targeting {}",
            card_name_for_history(*target, visible_tokens)
        ),
        SevenPlay::OneOff { target } => {
            format!(
                " as a one-off{}",
                oneoff_target_text(target, seat_names, visible_tokens)
            )
        }
        SevenPlay::Discard => " as discard".to_string(),
    }
}

fn card_name_for_history(card: Card, visible_tokens: &HashSet<String>) -> String {
    let token = card.to_token();
    if !visible_tokens.contains(&token) {
        return "Unknown card".to_string();
    }
    card_token_to_human(&token)
}

fn card_token_to_human(token: &str) -> String {
    if token == "J0" {
        return "Joker 0".to_string();
    }
    if token == "J1" {
        return "Joker 1".to_string();
    }
    let mut chars = token.chars();
    let rank = chars.next().unwrap_or('?');
    let suit = match chars.next().unwrap_or('?') {
        'C' => '♣',
        'D' => '♦',
        'H' => '♥',
        'S' => '♠',
        _ => '?',
    };
    format!("{}{}", rank, suit)
}

#[derive(Debug)]
enum Command {
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

async fn store_task(
    mut rx: mpsc::Receiver<Command>,
    persistence_tx: mpsc::Sender<CompletedGameRecord>,
    updates: broadcast::Sender<GameUpdate>,
    lobby_updates: broadcast::Sender<LobbyListUpdate>,
    scrap_straighten_updates: broadcast::Sender<ScrapStraightenUpdate>,
) {
    let mut store = Store::new(updates, lobby_updates, scrap_straighten_updates);
    while let Some(cmd) = rx.recv().await {
        match cmd {
            Command::CreateGame { user, respond } => {
                let id = store.create_game(user);
                let _ = respond.send(id);
            }
            Command::JoinGame {
                game_id,
                user,
                respond,
            } => {
                let result = store.join_game(game_id, user);
                let _ = respond.send(result);
            }
            Command::LeaveGame {
                game_id,
                user,
                respond,
            } => {
                let result = store.leave_game(game_id, user);
                let _ = respond.send(result);
            }
            Command::RematchGame {
                game_id,
                user,
                respond,
            } => {
                let result = store.rematch_game(game_id, user);
                let _ = respond.send(result);
            }
            Command::SetReady {
                game_id,
                user,
                ready,
                respond,
            } => {
                let result = store.set_ready(game_id, user, ready);
                let _ = respond.send(result);
            }
            Command::StartGame { game_id, respond } => {
                let result = store.start_game(game_id);
                let _ = respond.send(result);
            }
            Command::GetState {
                game_id,
                user,
                spectate_intent,
                respond,
            } => {
                let result = store.build_state_response_for_user(game_id, &user, spectate_intent);
                let _ = respond.send(result);
            }
            Command::ValidateViewer {
                game_id,
                user,
                spectate_intent,
                respond,
            } => {
                let result = store.validate_viewer(game_id, &user, spectate_intent);
                let _ = respond.send(result);
            }
            Command::SpectatorConnected {
                game_id,
                user,
                respond,
            } => {
                let result = store.spectator_connected(game_id, user);
                let _ = respond.send(result);
            }
            Command::SpectatorDisconnected { game_id, user_id } => {
                store.spectator_disconnected(game_id, user_id);
            }
            Command::ApplyAction {
                game_id,
                user,
                expected_version,
                action,
                respond,
            } => {
                let result = store.apply_action(game_id, user, expected_version, action);
                match result {
                    Ok((state, completed_record)) => {
                        if let Some(record) = completed_record
                            && persistence_tx.try_send(record).is_err()
                        {
                            error!(
                                "persistence worker channel full/closed; dropping completed game record"
                            );
                        }
                        let _ = respond.send(Ok(state));
                    }
                    Err(err) => {
                        let _ = respond.send(Err(err));
                    }
                }
            }
            Command::ToggleScrapStraighten {
                game_id,
                user,
                respond,
            } => {
                let result = store.toggle_scrap_straighten(game_id, user);
                let _ = respond.send(result);
            }
            Command::GetLobbyListForUser { user_id, respond } => {
                let _ = respond.send(store.lobby_list_for_user(Some(user_id)));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        SeatEntry, resolve_auto_run_migrations_from, resolve_database_url_from,
        usernames_from_seats,
    };

    #[test]
    fn resolve_database_url_prefers_cutthroat_specific_url() {
        let resolved = resolve_database_url_from(
            Some("postgres://cutthroat".to_string()),
            Some("postgres://fallback".to_string()),
        )
        .expect("database url");
        assert_eq!(resolved, "postgres://cutthroat");
    }

    #[test]
    fn resolve_database_url_uses_fallback_when_primary_missing() {
        let resolved = resolve_database_url_from(None, Some("postgres://fallback".to_string()))
            .expect("database url");
        assert_eq!(resolved, "postgres://fallback");
    }

    #[test]
    fn resolve_database_url_requires_any_url() {
        let err = resolve_database_url_from(None, None).expect_err("expected failure");
        assert!(
            err.to_string().contains("CUTTHROAT_DATABASE_URL"),
            "error should explain required env vars"
        );
    }

    #[test]
    fn auto_run_migrations_defaults_to_false() {
        assert!(!resolve_auto_run_migrations_from(None));
    }

    #[test]
    fn auto_run_migrations_enabled_only_for_true() {
        assert!(resolve_auto_run_migrations_from(Some("true".to_string())));
        assert!(resolve_auto_run_migrations_from(Some(" TRUE ".to_string())));
        assert!(!resolve_auto_run_migrations_from(Some("1".to_string())));
        assert!(!resolve_auto_run_migrations_from(Some("false".to_string())));
    }

    #[test]
    fn usernames_from_seats_maps_by_seat_index() {
        let seats = vec![
            SeatEntry {
                seat: 2,
                user_id: 12,
                username: "carol".to_string(),
                ready: true,
            },
            SeatEntry {
                seat: 0,
                user_id: 10,
                username: "alice".to_string(),
                ready: true,
            },
            SeatEntry {
                seat: 1,
                user_id: 11,
                username: "bob".to_string(),
                ready: true,
            },
        ];
        let names = usernames_from_seats(&seats).expect("expected full seat map");
        assert_eq!(
            names,
            ["alice".to_string(), "bob".to_string(), "carol".to_string()]
        );
    }
}
