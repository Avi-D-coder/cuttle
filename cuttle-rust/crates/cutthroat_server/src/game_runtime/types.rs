use crate::api::handlers::{GameStateResponse, LobbySummary, SpectatableGameSummary};
use crate::game_runtime::{GameCommand, STATUS_FINISHED, STATUS_LOBBY, STATUS_STARTED};
use axum::http::StatusCode;
use chrono::{DateTime, Utc};
use cutthroat_engine::{CutthroatState, LastEventView, Seat, TokenLog, Winner};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, watch};

#[derive(Clone, Debug)]
pub(crate) struct SeatEntry {
    pub(crate) seat: Seat,
    pub(crate) user_id: i64,
    pub(crate) username: String,
    pub(crate) ready: bool,
}

#[derive(Clone, Debug)]
pub(crate) struct GameEntry {
    pub(crate) id: i64,
    pub(crate) name: String,
    pub(crate) status: i16,
    pub(crate) is_rematch_lobby: bool,
    pub(crate) rematch_from_game_id: Option<i64>,
    pub(crate) series_anchor_game_id: i64,
    pub(crate) series_player_order: Vec<i64>,
    pub(crate) seats: Vec<SeatEntry>,
    pub(crate) transcript: TokenLog,
    pub(crate) last_event: Option<LastEventView>,
    pub(crate) scrap_straightened: bool,
    pub(crate) started_at: DateTime<Utc>,
    pub(crate) finished_at: DateTime<Utc>,
    pub(crate) active_spectators: HashMap<i64, (String, usize)>,
    pub(crate) version: i64,
    pub(crate) engine: CutthroatState,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum GameAudience {
    Seat(Seat),
    Spectator,
}

#[derive(Debug)]
pub(crate) struct GameStreamSubscription {
    pub(crate) audience: GameAudience,
    pub(crate) rx: watch::Receiver<Arc<GameStateResponse>>,
}

#[derive(Clone, Debug)]
pub(crate) struct LobbySummaryInternal {
    pub(crate) summary: LobbySummary,
    pub(crate) is_rematch_lobby: bool,
    pub(crate) seat_user_ids: Vec<i64>,
}

#[derive(Clone, Debug)]
pub(crate) struct LobbySnapshotInternal {
    pub(crate) version: u64,
    pub(crate) lobbies: Vec<LobbySummaryInternal>,
    pub(crate) spectatable_games: Vec<SpectatableGameSummary>,
}

#[derive(Debug)]
pub(crate) enum RuntimeError {
    NotFound,
    Forbidden,
    Conflict,
    BadRequest,
}

impl RuntimeError {
    pub(crate) fn status_code(&self) -> StatusCode {
        match self {
            RuntimeError::NotFound => StatusCode::NOT_FOUND,
            RuntimeError::Forbidden => StatusCode::FORBIDDEN,
            RuntimeError::Conflict => StatusCode::CONFLICT,
            RuntimeError::BadRequest => StatusCode::BAD_REQUEST,
        }
    }

    pub(crate) fn code(&self) -> u16 {
        self.status_code().as_u16()
    }

    pub(crate) fn message(&self) -> String {
        match self {
            RuntimeError::NotFound => "not found".to_string(),
            RuntimeError::Forbidden => "forbidden".to_string(),
            RuntimeError::Conflict => "conflict".to_string(),
            RuntimeError::BadRequest => "bad request".to_string(),
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct LobbyCacheEntry {
    pub(crate) game_id: i64,
    pub(crate) name: String,
    pub(crate) status: i16,
    pub(crate) seat_count: usize,
    pub(crate) ready_count: usize,
    pub(crate) is_rematch_lobby: bool,
    pub(crate) seat_user_ids: Vec<i64>,
    pub(crate) spectating_usernames: Vec<String>,
}

impl LobbyCacheEntry {
    pub(crate) fn from_game(game: &GameEntry) -> Self {
        let seat_user_ids = if game.is_rematch_lobby {
            game.series_player_order.clone()
        } else {
            game.seats.iter().map(|seat| seat.user_id).collect()
        };

        Self {
            game_id: game.id,
            name: game.name.clone(),
            status: game.status,
            seat_count: game.seats.len(),
            ready_count: game.seats.iter().filter(|seat| seat.ready).count(),
            is_rematch_lobby: game.is_rematch_lobby,
            seat_user_ids,
            spectating_usernames: active_spectator_usernames(game),
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct GameMeta {
    pub(crate) id: i64,
    pub(crate) status: i16,
    pub(crate) dealer: Seat,
    pub(crate) rematch_from_game_id: Option<i64>,
    pub(crate) series_anchor_game_id: i64,
    pub(crate) series_player_order: Vec<i64>,
    pub(crate) seats: Vec<SeatEntry>,
    pub(crate) winner_user_id: Option<i64>,
}

impl GameMeta {
    pub(crate) fn from_game(game: &GameEntry) -> Self {
        Self {
            id: game.id,
            status: game.status,
            dealer: game.transcript.dealer,
            rematch_from_game_id: game.rematch_from_game_id,
            series_anchor_game_id: game.series_anchor_game_id,
            series_player_order: game.series_player_order.clone(),
            seats: game.seats.clone(),
            winner_user_id: winner_user_id(game),
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct GameHandle {
    pub(crate) tx: mpsc::Sender<GameCommand>,
}

pub(crate) struct GlobalRuntimeState {
    pub(crate) next_id: i64,
    pub(crate) games: HashMap<i64, GameHandle>,
    pub(crate) rematches: HashMap<i64, i64>,
    pub(crate) lobby_cache: HashMap<i64, LobbyCacheEntry>,
    pub(crate) game_meta: HashMap<i64, GameMeta>,
    pub(crate) lobby_tx: watch::Sender<Arc<LobbySnapshotInternal>>,
}

impl GlobalRuntimeState {
    pub(crate) fn new(initial_next_game_id: i64) -> Self {
        let (lobby_tx, _lobby_rx) = watch::channel(Arc::new(LobbySnapshotInternal {
            version: 0,
            lobbies: Vec::new(),
            spectatable_games: Vec::new(),
        }));

        Self {
            next_id: initial_next_game_id.max(1),
            games: HashMap::new(),
            rematches: HashMap::new(),
            lobby_cache: HashMap::new(),
            game_meta: HashMap::new(),
            lobby_tx,
        }
    }

    fn build_lobby_snapshot(&self) -> LobbySnapshotInternal {
        let mut lobbies: Vec<LobbySummaryInternal> = self
            .lobby_cache
            .values()
            .filter(|entry| entry.status == STATUS_LOBBY)
            .map(|entry| LobbySummaryInternal {
                summary: LobbySummary {
                    id: entry.game_id,
                    name: entry.name.clone(),
                    seat_count: entry.seat_count,
                    ready_count: entry.ready_count,
                    status: entry.status,
                    viewer_has_reserved_seat: false,
                },
                is_rematch_lobby: entry.is_rematch_lobby,
                seat_user_ids: entry.seat_user_ids.clone(),
            })
            .collect();
        lobbies.sort_by_key(|entry| entry.summary.id);

        let mut spectatable_games: Vec<SpectatableGameSummary> = self
            .lobby_cache
            .values()
            .filter(|entry| entry.status == STATUS_STARTED)
            .map(|entry| SpectatableGameSummary {
                id: entry.game_id,
                name: entry.name.clone(),
                seat_count: entry.seat_count,
                status: entry.status,
                rematch_from_game_id: self
                    .game_meta
                    .get(&entry.game_id)
                    .and_then(|meta| meta.rematch_from_game_id),
                spectating_usernames: entry.spectating_usernames.clone(),
            })
            .collect();
        spectatable_games.sort_by_key(|entry| entry.id);

        LobbySnapshotInternal {
            // Kept for wire compatibility with existing frontend message shape.
            version: 0,
            lobbies,
            spectatable_games,
        }
    }

    pub(crate) fn publish_lobby_watch(&mut self) {
        let snapshot = Arc::new(self.build_lobby_snapshot());
        self.lobby_tx.send_replace(snapshot);
    }

    pub(crate) fn subscribe_lobby_stream(&self) -> watch::Receiver<Arc<LobbySnapshotInternal>> {
        self.lobby_tx.subscribe()
    }

    pub(crate) fn upsert_game_state(&mut self, game: &GameEntry) {
        self.lobby_cache
            .insert(game.id, LobbyCacheEntry::from_game(game));
        self.game_meta.insert(game.id, GameMeta::from_game(game));
    }

    pub(crate) fn remove_game(&mut self, game_id: i64) {
        self.games.remove(&game_id);
        self.lobby_cache.remove(&game_id);
        self.rematches.remove(&game_id);
        self.rematches
            .retain(|_, rematch_id| *rematch_id != game_id);
    }

    pub(crate) fn rematch_series_name(&self, prior_game_id: i64, series_order: &[i64]) -> String {
        let mut chain_ids = Vec::new();
        let mut cursor = Some(prior_game_id);
        while let Some(game_id) = cursor {
            let Some(meta) = self.game_meta.get(&game_id) else {
                break;
            };
            chain_ids.push(meta.id);
            cursor = meta.rematch_from_game_id;
        }
        chain_ids.reverse();

        let mut wins = [0usize; 3];
        let mut stalemates = 0usize;
        for game_id in chain_ids {
            let Some(meta) = self.game_meta.get(&game_id) else {
                continue;
            };
            if meta.status != STATUS_FINISHED {
                continue;
            }
            match meta.winner_user_id {
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
            .map(|user_id| {
                self.username_for_user_id(*user_id)
                    .unwrap_or_else(|| format!("User {}", user_id))
            })
            .collect();
        let n0 = names.first().cloned().unwrap_or_else(|| "P1".to_string());
        let n1 = names.get(1).cloned().unwrap_or_else(|| "P2".to_string());
        let n2 = names.get(2).cloned().unwrap_or_else(|| "P3".to_string());

        format!(
            "{} VS {} VS {} {}-{}-{}-{}",
            n0, n1, n2, wins[0], wins[1], wins[2], stalemates
        )
    }

    fn username_for_user_id(&self, user_id: i64) -> Option<String> {
        self.game_meta
            .values()
            .flat_map(|meta| meta.seats.iter())
            .find(|seat| seat.user_id == user_id)
            .map(|seat| seat.username.clone())
    }
}

pub(crate) fn active_spectator_usernames(game: &GameEntry) -> Vec<String> {
    let mut names: Vec<String> = game
        .active_spectators
        .values()
        .filter(|(_, count)| *count > 0)
        .map(|(username, _)| username.clone())
        .collect();
    names.sort();
    names
}

pub(crate) fn winner_user_id(game: &GameEntry) -> Option<i64> {
    let winner_seat = match game.engine.winner.as_ref()? {
        Winner::Seat(seat) => *seat,
        Winner::Draw => return None,
    };

    game.seats
        .iter()
        .find(|seat| seat.seat == winner_seat)
        .map(|seat| seat.user_id)
}
