use crate::auth::{AuthUser, authorize};
use crate::game_runtime::{
    GameCommand, GameEntry, GameStreamSubscription, LobbySnapshotInternal, STATUS_FINISHED,
    STATUS_STARTED, SeatEntry, create_game_for_user, create_rematch_for_user, game_sender,
};
#[cfg(feature = "e2e-seed")]
use crate::game_runtime::{
    SeedGameFromTranscriptInput, SeedGameInput, SeedGameResult, SeedSeatInput,
    seed_game_from_tokenlog as seed_game_from_tokenlog_runtime,
    seed_game_from_transcript as seed_game_from_transcript_runtime,
};
use crate::state::AppState;
use crate::view::history::build_history_log_for_viewer_with_limit;
use crate::view::response::{
    build_spectator_view, legal_action_tokens_for_seat, redact_tokenlog_for_client,
};
use axum::{
    Json,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
};
use chrono::{DateTime, Utc};
use cutthroat_engine::{Phase, PublicView, Seat, Winner, parse_tokenlog, replay_tokenlog};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot, watch};

#[derive(Deserialize)]
pub(crate) struct ActionRequest {
    pub(crate) expected_version: i64,
    pub(crate) action_tokens: String,
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
    pub(crate) viewer_has_reserved_seat: bool,
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
    pub(crate) legal_actions: Vec<String>,
    pub(crate) lobby: LobbyView,
    pub(crate) log_tail: Vec<String>,
    pub(crate) tokenlog: String,
    pub(crate) replay_total_states: i64,
    pub(crate) is_spectator: bool,
    pub(crate) spectating_usernames: Vec<String>,
    pub(crate) scrap_straightened: bool,
    pub(crate) archived: bool,
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

#[derive(Deserialize, Debug)]
pub(crate) struct HistoryQuery {
    pub(crate) limit: Option<usize>,
    pub(crate) before_finished_at: Option<String>,
    pub(crate) before_rust_game_id: Option<i64>,
}

#[derive(Deserialize, Debug, Default)]
pub(crate) struct SpectateStateQuery {
    #[serde(rename = "gameStateIndex")]
    pub(crate) game_state_index: Option<i64>,
}

fn action_seat_for_phase(phase: &Phase, turn: Seat) -> Seat {
    match phase {
        Phase::Countering(counter) => counter.next_seat,
        Phase::ResolvingThree { seat, .. }
        | Phase::ResolvingFour { seat, .. }
        | Phase::ResolvingFive { seat, .. }
        | Phase::ResolvingSeven { seat, .. } => *seat,
        _ => turn,
    }
}

#[derive(Serialize, Clone, Debug)]
pub(crate) struct HistoryPlayer {
    pub(crate) seat: Seat,
    pub(crate) user_id: i64,
    pub(crate) username: String,
}

#[derive(Serialize, Clone, Debug)]
pub(crate) struct HistoryGame {
    pub(crate) rust_game_id: i64,
    pub(crate) name: String,
    pub(crate) finished_at: DateTime<Utc>,
    pub(crate) winner_user_id: Option<i64>,
    pub(crate) viewer_won: Option<bool>,
    pub(crate) players: Vec<HistoryPlayer>,
    pub(crate) mode: &'static str,
}

#[derive(Serialize, Clone, Debug)]
pub(crate) struct HistoryResponse {
    pub(crate) finished_games: Vec<HistoryGame>,
    pub(crate) has_more: bool,
    pub(crate) next_cursor: Option<HistoryCursor>,
}

#[derive(Serialize, Clone, Debug)]
pub(crate) struct HistoryCursor {
    pub(crate) before_finished_at: String,
    pub(crate) before_rust_game_id: i64,
}

#[derive(Clone, Debug, FromRow)]
struct PersistedCutthroatGameRow {
    rust_game_id: i64,
    tokenlog: String,
    p0_user_id: i64,
    p1_user_id: i64,
    p2_user_id: i64,
    p0_username: String,
    p1_username: String,
    p2_username: String,
    started_at: DateTime<Utc>,
    finished_at: DateTime<Utc>,
}

fn db_pool(state: &AppState) -> Result<&PgPool, StatusCode> {
    state.db.as_ref().ok_or(StatusCode::SERVICE_UNAVAILABLE)
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
pub(crate) struct SeedGameFromTranscriptRequest {
    pub(crate) game_id: i64,
    pub(crate) players: Vec<SeedSeatFromTokenlogRequest>,
    pub(crate) dealer_seat: Seat,
    pub(crate) deck_tokens: Vec<String>,
    pub(crate) action_tokens: Vec<String>,
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

#[cfg(feature = "e2e-seed")]
pub(crate) type SeedGameFromTranscriptResponse = SeedGameFromTokenlogResponse;

pub(crate) async fn get_health() -> Json<HealthResponse> {
    Json(HealthResponse {
        alive: true,
        service: "cutthroat",
        version: env!("CARGO_PKG_VERSION"),
    })
}

pub(crate) async fn get_history(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<HistoryQuery>,
) -> Result<Json<HistoryResponse>, StatusCode> {
    let user = authorize(&state, &headers).await?;
    let pool = db_pool(&state)?;
    let limit = query.limit.unwrap_or(20).clamp(1, 50);
    let before_finished_at = query
        .before_finished_at
        .as_deref()
        .map(DateTime::parse_from_rfc3339)
        .transpose()
        .map_err(|_| StatusCode::BAD_REQUEST)?
        .map(|dt| dt.with_timezone(&Utc));
    let before_rust_game_id = query.before_rust_game_id;

    if before_finished_at.is_some() ^ before_rust_game_id.is_some() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let rows = sqlx::query_as::<_, PersistedCutthroatGameRow>(
        r#"
        SELECT
          cg.rust_game_id, cg.tokenlog,
          cg.p0_user_id, cg.p1_user_id, cg.p2_user_id,
          COALESCE(u0.username, CONCAT('User ', cg.p0_user_id::text)) AS p0_username,
          COALESCE(u1.username, CONCAT('User ', cg.p1_user_id::text)) AS p1_username,
          COALESCE(u2.username, CONCAT('User ', cg.p2_user_id::text)) AS p2_username,
          cg.started_at, cg.finished_at
        FROM cutthroat_games cg
        LEFT JOIN "user" u0 ON u0.id = cg.p0_user_id
        LEFT JOIN "user" u1 ON u1.id = cg.p1_user_id
        LEFT JOIN "user" u2 ON u2.id = cg.p2_user_id
        WHERE ($1 = cg.p0_user_id OR $1 = cg.p1_user_id OR $1 = cg.p2_user_id)
          AND (
            $2::timestamptz IS NULL
            OR (cg.finished_at, cg.rust_game_id) < ($2::timestamptz, $3::bigint)
          )
        ORDER BY cg.finished_at DESC, cg.rust_game_id DESC
        LIMIT $4
        "#,
    )
    .bind(user.id)
    .bind(before_finished_at)
    .bind(before_rust_game_id)
    .bind((limit + 1) as i64)
    .fetch_all(pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let has_more = rows.len() > limit;
    let page_rows = rows.into_iter().take(limit).collect::<Vec<_>>();
    let games = page_rows
        .iter()
        .map(|row| build_history_game_from_row(row, user.id))
        .collect::<Vec<_>>();
    let next_cursor = page_rows.last().and_then(|row| {
        if !has_more {
            return None;
        }
        Some(HistoryCursor {
            before_finished_at: row.finished_at.to_rfc3339(),
            before_rust_game_id: row.rust_game_id,
        })
    });

    Ok(Json(HistoryResponse {
        finished_games: games,
        has_more,
        next_cursor,
    }))
}

#[cfg(feature = "e2e-seed")]
pub(crate) async fn seed_game_from_tokenlog(
    State(state): State<AppState>,
    Json(body): Json<SeedGameFromTokenlogRequest>,
) -> Result<Json<SeedGameFromTokenlogResponse>, StatusCode> {
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
    let SeedGameResult {
        game_id,
        version,
        status,
        seat_user_ids,
        tokenlog,
        created,
        replaced_existing,
    } = seed_game_from_tokenlog_runtime(state.runtime.clone(), state.persistence_tx.clone(), seed)
        .await
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

#[cfg(feature = "e2e-seed")]
pub(crate) async fn seed_game_from_transcript(
    State(state): State<AppState>,
    Json(body): Json<SeedGameFromTranscriptRequest>,
) -> Result<Json<SeedGameFromTranscriptResponse>, StatusCode> {
    let seed = SeedGameFromTranscriptInput {
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
        deck_tokens: body.deck_tokens,
        action_tokens: body.action_tokens,
        status: body.status,
        spectating_usernames: body.spectating_usernames,
        name: body.name,
    };
    let SeedGameResult {
        game_id,
        version,
        status,
        seat_user_ids,
        tokenlog,
        created,
        replaced_existing,
    } = seed_game_from_transcript_runtime(
        state.runtime.clone(),
        state.persistence_tx.clone(),
        seed,
    )
    .await
    .map_err(|err| err.status_code())?;

    Ok(Json(SeedGameFromTranscriptResponse {
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
    let id = create_game_for_user(state.runtime.clone(), state.persistence_tx.clone(), user).await;
    Ok(Json(serde_json::json!({ "id": id })))
}

pub(crate) async fn join_game(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let user = authorize(&state, &headers).await?;
    let sender = game_sender(&state.runtime, id)
        .await
        .ok_or(StatusCode::NOT_FOUND)?;
    let (tx, rx) = oneshot::channel();
    sender
        .send(GameCommand::JoinGame { user, respond: tx })
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;
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
    let rematch_id = create_rematch_for_user(
        state.runtime.clone(),
        state.persistence_tx.clone(),
        id,
        user,
    )
    .await
    .map_err(|err| err.status_code())?;
    Ok(Json(serde_json::json!({ "id": rematch_id })))
}

pub(crate) async fn leave_game(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    headers: HeaderMap,
) -> Result<StatusCode, StatusCode> {
    let user = authorize(&state, &headers).await?;
    let sender = game_sender(&state.runtime, id)
        .await
        .ok_or(StatusCode::NOT_FOUND)?;
    let (tx, rx) = oneshot::channel();
    sender
        .send(GameCommand::LeaveGame { user, respond: tx })
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;
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
    let sender = game_sender(&state.runtime, id)
        .await
        .ok_or(StatusCode::NOT_FOUND)?;
    let (tx, rx) = oneshot::channel();
    sender
        .send(GameCommand::SetReady {
            user,
            ready: body.ready,
            respond: tx,
        })
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;
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
    let user = authorize(&state, &headers).await?;
    let sender = game_sender(&state.runtime, id)
        .await
        .ok_or(StatusCode::NOT_FOUND)?;
    let (tx, rx) = oneshot::channel();
    sender
        .send(GameCommand::StartGame { user, respond: tx })
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;
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
    get_state_inner(state, id, headers, false, None).await
}

pub(crate) async fn get_spectate_state(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    headers: HeaderMap,
    Query(query): Query<SpectateStateQuery>,
) -> Result<Json<GameStateResponse>, StatusCode> {
    get_state_inner(state, id, headers, true, query.game_state_index).await
}

async fn get_state_inner(
    state: AppState,
    id: i64,
    headers: HeaderMap,
    spectate_intent: bool,
    game_state_index: Option<i64>,
) -> Result<Json<GameStateResponse>, StatusCode> {
    let user = authorize(&state, &headers).await?;
    let user_for_fallback = user.clone();
    let normalized_game_state_index = game_state_index.unwrap_or(-1).max(-1);
    let sender = game_sender(&state.runtime, id).await;

    if sender.is_none() {
        if spectate_intent
            && let Some(resp) = load_archived_spectate_state(
                &state,
                id,
                &user_for_fallback,
                normalized_game_state_index,
            )
            .await?
        {
            return Ok(Json(resp));
        }
        return Err(StatusCode::NOT_FOUND);
    }
    let sender = sender.expect("checked is_some");

    let (tx, rx) = oneshot::channel();
    if spectate_intent && normalized_game_state_index >= 0 {
        sender
            .send(GameCommand::GetSpectateReplayState {
                user,
                game_state_index: normalized_game_state_index,
                respond: tx,
            })
            .await
            .map_err(|_| StatusCode::NOT_FOUND)?;
    } else {
        sender
            .send(GameCommand::GetState {
                user,
                spectate_intent,
                respond: tx,
            })
            .await
            .map_err(|_| StatusCode::NOT_FOUND)?;
    }
    let runtime_result = rx.await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    match runtime_result {
        Ok(resp) => Ok(Json(resp)),
        Err(err) => {
            if spectate_intent
                && err.status_code() == StatusCode::NOT_FOUND
                && let Some(resp) = load_archived_spectate_state(
                    &state,
                    id,
                    &user_for_fallback,
                    normalized_game_state_index,
                )
                .await?
            {
                return Ok(Json(resp));
            }
            Err(err.status_code())
        }
    }
}

async fn load_archived_spectate_state(
    state: &AppState,
    id: i64,
    user: &AuthUser,
    game_state_index: i64,
) -> Result<Option<GameStateResponse>, StatusCode> {
    let pool = db_pool(state)?;
    let row = sqlx::query_as::<_, PersistedCutthroatGameRow>(
        r#"
        SELECT
          cg.rust_game_id, cg.tokenlog,
          cg.p0_user_id, cg.p1_user_id, cg.p2_user_id,
          COALESCE(u0.username, CONCAT('User ', cg.p0_user_id::text)) AS p0_username,
          COALESCE(u1.username, CONCAT('User ', cg.p1_user_id::text)) AS p1_username,
          COALESCE(u2.username, CONCAT('User ', cg.p2_user_id::text)) AS p2_username,
          cg.started_at, cg.finished_at
        FROM cutthroat_games cg
        LEFT JOIN "user" u0 ON u0.id = cg.p0_user_id
        LEFT JOIN "user" u1 ON u1.id = cg.p1_user_id
        LEFT JOIN "user" u2 ON u2.id = cg.p2_user_id
        WHERE cg.rust_game_id = $1
        LIMIT 1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let Some(row) = row else {
        return Ok(None);
    };
    if user.id != row.p0_user_id && user.id != row.p1_user_id && user.id != row.p2_user_id {
        return Err(StatusCode::FORBIDDEN);
    }

    let game = build_game_entry_from_row(&row).ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let parsed = game.transcript.clone();
    let replay_index = if game_state_index < 0 {
        parsed.actions.len()
    } else {
        usize::try_from(game_state_index).map_err(|_| StatusCode::BAD_REQUEST)?
    };
    if replay_index > parsed.actions.len() {
        return Err(StatusCode::NOT_FOUND);
    }

    let mut truncated = parsed.clone();
    truncated.actions.truncate(replay_index);
    let replayed = replay_tokenlog(&truncated).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut replay_game = game.clone();
    replay_game.engine = replayed;
    replay_game.version = replay_index as i64;
    replay_game.last_event = None;
    replay_game.scrap_straightened = false;
    replay_game.status = if replay_index < parsed.actions.len() {
        STATUS_STARTED
    } else {
        STATUS_FINISHED
    };

    let spectator_view = build_spectator_view(&replay_game);
    let log_tail = build_history_log_for_viewer_with_limit(&replay_game, 0, Some(replay_index));
    let action_seat = action_seat_for_phase(&replay_game.engine.phase, replay_game.engine.turn);
    let legal_actions = if replay_game.status == STATUS_STARTED {
        legal_action_tokens_for_seat(&replay_game.engine, action_seat)
    } else {
        Vec::new()
    };
    let tokenlog = redact_tokenlog_for_client(&game.transcript, None);
    Ok(Some(GameStateResponse {
        version: replay_game.version,
        seat: 0,
        status: replay_game.status,
        player_view: spectator_view.clone(),
        spectator_view,
        legal_actions,
        lobby: LobbyView {
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
        },
        log_tail,
        tokenlog,
        replay_total_states: game.transcript.actions.len() as i64 + 1,
        is_spectator: true,
        spectating_usernames: Vec::new(),
        scrap_straightened: false,
        archived: true,
    }))
}

fn build_game_entry_from_row(row: &PersistedCutthroatGameRow) -> Option<GameEntry> {
    let parsed = parse_tokenlog(&row.tokenlog).ok()?;
    let action_count = parsed.actions.len() as i64;
    let engine = replay_tokenlog(&parsed).ok()?;
    Some(GameEntry {
        id: row.rust_game_id,
        name: format!(
            "{} VS {} VS {}",
            row.p0_username, row.p1_username, row.p2_username
        ),
        status: STATUS_FINISHED,
        is_rematch_lobby: false,
        rematch_from_game_id: None,
        series_anchor_game_id: row.rust_game_id,
        series_player_order: vec![row.p0_user_id, row.p1_user_id, row.p2_user_id],
        seats: vec![
            SeatEntry {
                seat: 0,
                user_id: row.p0_user_id,
                username: row.p0_username.clone(),
                ready: true,
            },
            SeatEntry {
                seat: 1,
                user_id: row.p1_user_id,
                username: row.p1_username.clone(),
                ready: true,
            },
            SeatEntry {
                seat: 2,
                user_id: row.p2_user_id,
                username: row.p2_username.clone(),
                ready: true,
            },
        ],
        transcript: parsed,
        last_event: None,
        scrap_straightened: false,
        started_at: row.started_at,
        finished_at: row.finished_at,
        active_spectators: HashMap::new(),
        version: action_count,
        engine,
    })
}

fn build_history_game_from_row(
    row: &PersistedCutthroatGameRow,
    viewer_user_id: i64,
) -> HistoryGame {
    let winner_user_id = derive_winner_user_id(row);
    HistoryGame {
        rust_game_id: row.rust_game_id,
        name: format!(
            "{} VS {} VS {}",
            row.p0_username, row.p1_username, row.p2_username
        ),
        finished_at: row.finished_at,
        winner_user_id,
        viewer_won: winner_user_id.map(|winner_id| winner_id == viewer_user_id),
        players: vec![
            HistoryPlayer {
                seat: 0,
                user_id: row.p0_user_id,
                username: row.p0_username.clone(),
            },
            HistoryPlayer {
                seat: 1,
                user_id: row.p1_user_id,
                username: row.p1_username.clone(),
            },
            HistoryPlayer {
                seat: 2,
                user_id: row.p2_user_id,
                username: row.p2_username.clone(),
            },
        ],
        mode: "cutthroat",
    }
}

fn derive_winner_user_id(row: &PersistedCutthroatGameRow) -> Option<i64> {
    let parsed = parse_tokenlog(&row.tokenlog).ok()?;
    let state = replay_tokenlog(&parsed).ok()?;
    match state.winner {
        Some(Winner::Seat(0)) => Some(row.p0_user_id),
        Some(Winner::Seat(1)) => Some(row.p1_user_id),
        Some(Winner::Seat(2)) => Some(row.p2_user_id),
        Some(Winner::Draw) | None => None,
        Some(Winner::Seat(_)) => None,
    }
}

pub(crate) async fn post_action(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    headers: HeaderMap,
    Json(body): Json<ActionRequest>,
) -> Result<Json<GameStateResponse>, StatusCode> {
    let user = authorize(&state, &headers).await?;
    let sender = game_sender(&state.runtime, id)
        .await
        .ok_or(StatusCode::NOT_FOUND)?;
    let resp = apply_action_with_sender(&sender, user, body.expected_version, body.action_tokens)
        .await
        .map_err(|(code, _)| {
            StatusCode::from_u16(code).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)
        })?;
    Ok(Json(resp))
}

pub(crate) async fn apply_action_with_sender(
    sender: &mpsc::Sender<GameCommand>,
    user: AuthUser,
    expected_version: i64,
    action_tokens: String,
) -> Result<GameStateResponse, (u16, String)> {
    let (tx, rx) = oneshot::channel();
    sender
        .send(GameCommand::ApplyAction {
            user,
            expected_version,
            action_tokens,
            respond: tx,
        })
        .await
        .map_err(|_| (404, "not found".to_string()))?;

    rx.await
        .map_err(|_| (500, "server error".to_string()))?
        .map_err(|err| (err.code(), err.message()))
}

pub(crate) async fn toggle_scrap_straighten_with_sender(
    sender: &mpsc::Sender<GameCommand>,
    user: AuthUser,
) -> Result<(), (u16, String)> {
    let (tx, rx) = oneshot::channel();
    sender
        .send(GameCommand::ToggleScrapStraighten { user, respond: tx })
        .await
        .map_err(|_| (404, "not found".to_string()))?;

    rx.await
        .map_err(|_| (500, "server error".to_string()))?
        .map_err(|err| (err.code(), err.message()))
}

pub(crate) async fn subscribe_game_stream(
    state: &AppState,
    game_id: i64,
    user: AuthUser,
    spectate_intent: bool,
) -> Result<(mpsc::Sender<GameCommand>, GameStreamSubscription), (u16, String)> {
    let sender = game_sender(&state.runtime, game_id)
        .await
        .ok_or((404, "not found".to_string()))?;
    let (tx, rx) = oneshot::channel();
    sender
        .send(GameCommand::SubscribeGameStream {
            user,
            spectate_intent,
            respond: tx,
        })
        .await
        .map_err(|_| (404, "not found".to_string()))?;

    let subscription = rx
        .await
        .map_err(|_| (500, "server error".to_string()))?
        .map_err(|err| (err.code(), err.message()))?;
    Ok((sender, subscription))
}

pub(crate) async fn subscribe_lobby_stream(
    state: &AppState,
) -> Result<watch::Receiver<Arc<LobbySnapshotInternal>>, StatusCode> {
    let guard = state.runtime.read().await;
    Ok(guard.subscribe_lobby_stream())
}

pub(crate) async fn set_socket_disconnected(
    sender: &mpsc::Sender<GameCommand>,
    user_id: i64,
    audience: crate::game_runtime::GameAudience,
) {
    let _ = sender
        .send(GameCommand::SocketDisconnected { user_id, audience })
        .await;
}
