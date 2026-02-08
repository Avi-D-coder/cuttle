use crate::api::handlers::{
    GameStateResponse, LobbySeatView, LobbySummary, LobbyView, SpectatableGameSummary,
};
use crate::auth::AuthUser;
#[cfg(feature = "e2e-seed")]
use crate::game_runtime::commands::{SeedGameInput, SeedGameResult, SeedSeatInput};
use crate::game_runtime::{STATUS_FINISHED, STATUS_LOBBY, STATUS_STARTED};
use crate::persistence::CompletedGameRecord;
use crate::view::history::build_history_log_for_viewer;
use crate::view::response::{
    build_last_event, build_spectator_view, format_action, normal_lobby_name,
    redact_tokenlog_for_client,
};
use axum::http::StatusCode;
use chrono::{DateTime, Utc};
use cutthroat_engine::{
    Action, CutthroatState, LastEventView, Phase, Seat, Winner, append_action, encode_header,
};
#[cfg(feature = "e2e-seed")]
use cutthroat_engine::{parse_tokenlog, replay_tokenlog};
use rand::seq::SliceRandom;
use std::collections::HashMap;
#[cfg(feature = "e2e-seed")]
use std::collections::{BTreeMap, HashSet};
use std::sync::Arc;
use tokio::sync::watch;

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
    pub(crate) tokenlog_full: String,
    pub(crate) last_event: Option<LastEventView>,
    pub(crate) scrap_straightened: bool,
    pub(crate) started_at: Option<DateTime<Utc>>,
    pub(crate) finished_at: Option<DateTime<Utc>>,
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

struct GameWatchSet {
    seat_tx: [watch::Sender<Arc<GameStateResponse>>; 3],
    spectator_tx: watch::Sender<Arc<GameStateResponse>>,
}

pub(crate) struct GameRuntime {
    next_id: i64,
    lobby_version: u64,
    games: HashMap<i64, GameEntry>,
    rematches: HashMap<i64, i64>,
    game_streams: HashMap<i64, GameWatchSet>,
    lobby_tx: watch::Sender<Arc<LobbySnapshotInternal>>,
}

impl GameRuntime {
    pub(crate) fn new(initial_next_game_id: i64) -> Self {
        let (lobby_tx, _lobby_rx) = watch::channel(Arc::new(LobbySnapshotInternal {
            version: 0,
            lobbies: Vec::new(),
            spectatable_games: Vec::new(),
        }));

        Self {
            next_id: initial_next_game_id.max(1),
            lobby_version: 0,
            games: HashMap::new(),
            rematches: HashMap::new(),
            game_streams: HashMap::new(),
            lobby_tx,
        }
    }

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

    fn build_lobby_snapshot(&self, version: u64) -> LobbySnapshotInternal {
        let mut lobbies: Vec<LobbySummaryInternal> = self
            .games
            .values()
            .filter(|game| game.status == STATUS_LOBBY)
            .map(|game| LobbySummaryInternal {
                summary: LobbySummary {
                    id: game.id,
                    name: game.name.clone(),
                    seat_count: game.seats.len(),
                    ready_count: game.seats.iter().filter(|seat| seat.ready).count(),
                    status: game.status,
                },
                is_rematch_lobby: game.is_rematch_lobby,
                seat_user_ids: game.seats.iter().map(|seat| seat.user_id).collect(),
            })
            .collect();
        lobbies.sort_by_key(|lobby| lobby.summary.id);

        let mut spectatable_games: Vec<SpectatableGameSummary> = self
            .games
            .values()
            .filter(|game| game.status == STATUS_STARTED)
            .map(|game| SpectatableGameSummary {
                id: game.id,
                name: game.name.clone(),
                seat_count: game.seats.len(),
                status: game.status,
                spectating_usernames: Self::active_spectator_usernames(game),
            })
            .collect();
        spectatable_games.sort_by_key(|game| game.id);

        LobbySnapshotInternal {
            version,
            lobbies,
            spectatable_games,
        }
    }

    fn publish_lobby_watch(&mut self) {
        self.lobby_version = self.lobby_version.saturating_add(1);
        let snapshot = Arc::new(self.build_lobby_snapshot(self.lobby_version));
        self.lobby_tx.send_replace(snapshot);
    }

    pub(crate) fn subscribe_lobby_stream(&self) -> watch::Receiver<Arc<LobbySnapshotInternal>> {
        self.lobby_tx.subscribe()
    }

    fn create_game_watch_set(&self, game_id: i64) -> Result<GameWatchSet, RuntimeError> {
        let seat0 = Arc::new(self.build_state_response(game_id, 0)?);
        let seat1 = Arc::new(self.build_state_response(game_id, 1)?);
        let seat2 = Arc::new(self.build_state_response(game_id, 2)?);
        let game = self.games.get(&game_id).ok_or(RuntimeError::NotFound)?;
        let spectator = Arc::new(self.build_spectator_state_response(game));

        let (seat0_tx, _seat0_rx) = watch::channel(seat0);
        let (seat1_tx, _seat1_rx) = watch::channel(seat1);
        let (seat2_tx, _seat2_rx) = watch::channel(seat2);
        let (spectator_tx, _spectator_rx) = watch::channel(spectator);

        Ok(GameWatchSet {
            seat_tx: [seat0_tx, seat1_tx, seat2_tx],
            spectator_tx,
        })
    }

    fn publish_game_watch(&self, game_id: i64) {
        let Some(streams) = self.game_streams.get(&game_id) else {
            return;
        };
        let Ok(seat0) = self.build_state_response(game_id, 0) else {
            return;
        };
        let Ok(seat1) = self.build_state_response(game_id, 1) else {
            return;
        };
        let Ok(seat2) = self.build_state_response(game_id, 2) else {
            return;
        };
        let Some(game) = self.games.get(&game_id) else {
            return;
        };
        let spectator = self.build_spectator_state_response(game);

        streams.seat_tx[0].send_replace(Arc::new(seat0));
        streams.seat_tx[1].send_replace(Arc::new(seat1));
        streams.seat_tx[2].send_replace(Arc::new(seat2));
        streams.spectator_tx.send_replace(Arc::new(spectator));
    }

    pub(crate) fn create_game(&mut self, user: AuthUser) -> i64 {
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
        if let Ok(watch_set) = self.create_game_watch_set(id) {
            self.game_streams.insert(id, watch_set);
        }
        self.publish_lobby_watch();
        self.publish_game_watch(id);
        id
    }

    #[cfg(feature = "e2e-seed")]
    pub(crate) fn seed_game_from_tokenlog(
        &mut self,
        seed: SeedGameInput,
    ) -> Result<SeedGameResult, RuntimeError> {
        if seed.game_id <= 0 {
            return Err(RuntimeError::BadRequest);
        }
        if seed.players.is_empty() || seed.players.len() > 3 {
            return Err(RuntimeError::BadRequest);
        }

        let mut seen_seats: HashSet<Seat> = HashSet::new();
        let mut seen_users: HashSet<i64> = HashSet::new();
        for player in &seed.players {
            if player.seat >= 3 {
                return Err(RuntimeError::BadRequest);
            }
            if !seen_seats.insert(player.seat) || !seen_users.insert(player.user_id) {
                return Err(RuntimeError::BadRequest);
            }
            if player.username.trim().is_empty() {
                return Err(RuntimeError::BadRequest);
            }
        }

        let parsed = parse_tokenlog(&seed.tokenlog).map_err(|_| RuntimeError::BadRequest)?;
        if let Some(dealer_seat) = seed.dealer_seat
            && dealer_seat != parsed.dealer
        {
            return Err(RuntimeError::BadRequest);
        }
        let engine = replay_tokenlog(&parsed).map_err(|_| RuntimeError::BadRequest)?;

        let status = match seed.status {
            Some(STATUS_LOBBY | STATUS_STARTED | STATUS_FINISHED) => {
                seed.status.unwrap_or(STATUS_STARTED)
            }
            Some(_) => return Err(RuntimeError::BadRequest),
            None => {
                if engine.winner.is_some() {
                    STATUS_FINISHED
                } else {
                    STATUS_STARTED
                }
            }
        };

        let mut seats: Vec<SeatEntry> = seed
            .players
            .into_iter()
            .map(|player: SeedSeatInput| SeatEntry {
                seat: player.seat,
                user_id: player.user_id,
                username: player.username,
                ready: player.ready.unwrap_or(status != STATUS_LOBBY),
            })
            .collect();
        seats.sort_by_key(|entry| entry.seat);

        let name = seed
            .name
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| normal_lobby_name(&seats));

        let now = Utc::now();
        let started_at = if status == STATUS_STARTED || status == STATUS_FINISHED {
            Some(now)
        } else {
            None
        };
        let finished_at = if status == STATUS_FINISHED {
            Some(now)
        } else {
            None
        };

        let active_spectators = seed
            .spectating_usernames
            .unwrap_or_default()
            .into_iter()
            .enumerate()
            .map(|(idx, username)| (-(idx as i64) - 1, (username, 1usize)))
            .collect::<HashMap<_, _>>();

        let created = !self.games.contains_key(&seed.game_id);
        let replaced_existing = !created;
        if replaced_existing {
            self.games.remove(&seed.game_id);
            self.game_streams.remove(&seed.game_id);
            self.rematches
                .retain(|key, value| *key != seed.game_id && *value != seed.game_id);
        }

        let game = GameEntry {
            id: seed.game_id,
            name,
            status,
            is_rematch_lobby: false,
            rematch_from_game_id: None,
            series_anchor_game_id: seed.game_id,
            series_player_order: seats.iter().map(|entry| entry.user_id).collect(),
            seats,
            tokenlog_full: seed.tokenlog.clone(),
            last_event: None,
            scrap_straightened: false,
            started_at,
            finished_at,
            active_spectators,
            version: parsed.actions.len() as i64,
            engine,
        };

        self.games.insert(seed.game_id, game);
        if let Ok(watch_set) = self.create_game_watch_set(seed.game_id) {
            self.game_streams.insert(seed.game_id, watch_set);
        } else {
            return Err(RuntimeError::BadRequest);
        }
        if seed.game_id >= self.next_id {
            self.next_id = seed.game_id + 1;
        }
        self.publish_lobby_watch();
        self.publish_game_watch(seed.game_id);

        let game = self
            .games
            .get(&seed.game_id)
            .ok_or(RuntimeError::NotFound)?;
        let seat_user_ids = game
            .seats
            .iter()
            .map(|entry| (entry.seat.to_string(), entry.user_id))
            .collect::<BTreeMap<_, _>>();

        Ok(SeedGameResult {
            game_id: game.id,
            version: game.version,
            status: game.status,
            seat_user_ids,
            tokenlog: game.tokenlog_full.clone(),
            created,
            replaced_existing,
        })
    }

    pub(crate) fn rematch_game(
        &mut self,
        game_id: i64,
        user: AuthUser,
    ) -> Result<i64, RuntimeError> {
        let prior_game = self.games.get(&game_id).ok_or(RuntimeError::NotFound)?;
        if prior_game.status != STATUS_FINISHED {
            return Err(RuntimeError::Conflict);
        }
        if !prior_game.seats.iter().any(|seat| seat.user_id == user.id) {
            return Err(RuntimeError::Forbidden);
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
        if let Ok(watch_set) = self.create_game_watch_set(id) {
            self.game_streams.insert(id, watch_set);
        }
        self.rematches.insert(game_id, id);
        self.publish_lobby_watch();
        self.publish_game_watch(id);
        Ok(id)
    }

    pub(crate) fn join_game(&mut self, game_id: i64, user: AuthUser) -> Result<Seat, RuntimeError> {
        let joined_seat = {
            let game = self.games.get_mut(&game_id).ok_or(RuntimeError::NotFound)?;
            if game.status != STATUS_LOBBY {
                return Err(RuntimeError::Conflict);
            }

            if let Some(existing) = game.seats.iter().find(|seat| seat.user_id == user.id) {
                existing.seat
            } else {
                if game.seats.len() >= 3 {
                    return Err(RuntimeError::Conflict);
                }

                let mut occupied = [false; 3];
                for seat in &game.seats {
                    occupied[seat.seat as usize] = true;
                }
                let seat_index = occupied
                    .iter()
                    .position(|v| !*v)
                    .ok_or(RuntimeError::Conflict)?;

                game.seats.push(SeatEntry {
                    seat: seat_index as Seat,
                    user_id: user.id,
                    username: user.username,
                    ready: false,
                });
                game.name = normal_lobby_name(&game.seats);
                seat_index as Seat
            }
        };

        self.publish_lobby_watch();
        self.publish_game_watch(game_id);
        Ok(joined_seat)
    }

    pub(crate) fn leave_game(&mut self, game_id: i64, user: AuthUser) -> Result<(), RuntimeError> {
        let mut should_remove_game = false;
        {
            let game = self.games.get_mut(&game_id).ok_or(RuntimeError::NotFound)?;
            if game.status != STATUS_LOBBY {
                return Err(RuntimeError::Conflict);
            }
            let idx = game
                .seats
                .iter()
                .position(|seat| seat.user_id == user.id)
                .ok_or(RuntimeError::Forbidden)?;
            game.seats.remove(idx);
            if game.seats.is_empty() {
                should_remove_game = true;
            } else if !game.is_rematch_lobby {
                game.name = normal_lobby_name(&game.seats);
            }
        }

        if should_remove_game {
            self.games.remove(&game_id);
            self.game_streams.remove(&game_id);
            self.rematches
                .retain(|_, rematch_id| *rematch_id != game_id);
        } else {
            self.publish_game_watch(game_id);
        }
        self.publish_lobby_watch();
        Ok(())
    }

    pub(crate) fn set_ready(
        &mut self,
        game_id: i64,
        user: AuthUser,
        ready: bool,
    ) -> Result<(), RuntimeError> {
        {
            let game = self.games.get_mut(&game_id).ok_or(RuntimeError::NotFound)?;
            if game.status != STATUS_LOBBY {
                return Err(RuntimeError::Conflict);
            }
            let seat = game
                .seats
                .iter_mut()
                .find(|seat| seat.user_id == user.id)
                .ok_or(RuntimeError::Forbidden)?;
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
        }

        self.publish_lobby_watch();
        self.publish_game_watch(game_id);
        Ok(())
    }

    pub(crate) fn start_game(&mut self, game_id: i64) -> Result<(), RuntimeError> {
        {
            let game = self.games.get_mut(&game_id).ok_or(RuntimeError::NotFound)?;
            if game.status != STATUS_LOBBY {
                return Err(RuntimeError::Conflict);
            }
            if game.seats.len() != 3 || !game.seats.iter().all(|seat| seat.ready) {
                return Err(RuntimeError::Conflict);
            }
            game.status = STATUS_STARTED;
            if game.started_at.is_none() {
                game.started_at = Some(Utc::now());
            }
        }

        self.publish_lobby_watch();
        self.publish_game_watch(game_id);
        Ok(())
    }

    pub(crate) fn validate_viewer(
        &self,
        game_id: i64,
        user: &AuthUser,
        spectate_intent: bool,
    ) -> Result<(), RuntimeError> {
        let game = self.games.get(&game_id).ok_or(RuntimeError::NotFound)?;
        let viewer_is_seated = game.seats.iter().any(|seat| seat.user_id == user.id);
        if spectate_intent {
            if viewer_is_seated && game.status != STATUS_FINISHED {
                return Err(RuntimeError::Conflict);
            }
            if game.status != STATUS_STARTED && game.status != STATUS_FINISHED {
                return Err(RuntimeError::Conflict);
            }
            return Ok(());
        }

        if viewer_is_seated {
            return Ok(());
        }
        if game.status == STATUS_LOBBY {
            return Err(RuntimeError::Conflict);
        }
        Ok(())
    }

    pub(crate) fn subscribe_game_stream(
        &mut self,
        game_id: i64,
        user: AuthUser,
        spectate_intent: bool,
    ) -> Result<GameStreamSubscription, RuntimeError> {
        self.validate_viewer(game_id, &user, spectate_intent)?;

        let maybe_seat = self.games.get(&game_id).and_then(|game| {
            game.seats
                .iter()
                .find(|seat| seat.user_id == user.id)
                .map(|seat| seat.seat)
        });

        let audience = if spectate_intent || maybe_seat.is_none() {
            GameAudience::Spectator
        } else {
            GameAudience::Seat(maybe_seat.unwrap_or(0))
        };

        if matches!(audience, GameAudience::Spectator) {
            let game = self.games.get_mut(&game_id).ok_or(RuntimeError::NotFound)?;
            let entry = game
                .active_spectators
                .entry(user.id)
                .or_insert((user.username, 0));
            entry.1 += 1;
            self.publish_game_watch(game_id);
            self.publish_lobby_watch();
        }

        let streams = self
            .game_streams
            .get(&game_id)
            .ok_or(RuntimeError::NotFound)?;
        let rx = match audience {
            GameAudience::Spectator => streams.spectator_tx.subscribe(),
            GameAudience::Seat(seat) => {
                let idx = seat as usize;
                if idx >= 3 {
                    return Err(RuntimeError::BadRequest);
                }
                streams.seat_tx[idx].subscribe()
            }
        };

        Ok(GameStreamSubscription { audience, rx })
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
            scrap_straightened: game.scrap_straightened,
            archived: false,
        }
    }

    pub(crate) fn build_state_response_for_user(
        &self,
        game_id: i64,
        user: &AuthUser,
        spectate_intent: bool,
    ) -> Result<GameStateResponse, RuntimeError> {
        self.validate_viewer(game_id, user, spectate_intent)?;
        let game = self.games.get(&game_id).ok_or(RuntimeError::NotFound)?;
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
    ) -> Result<GameStateResponse, RuntimeError> {
        let game = self.games.get(&game_id).ok_or(RuntimeError::NotFound)?;
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
            scrap_straightened: game.scrap_straightened,
            archived: false,
        })
    }

    pub(crate) fn spectator_disconnected(&mut self, game_id: i64, user_id: i64) {
        let mut changed = false;
        if let Some(game) = self.games.get_mut(&game_id)
            && let Some((_, count)) = game.active_spectators.get_mut(&user_id)
        {
            if *count > 1 {
                *count -= 1;
                changed = true;
            } else {
                game.active_spectators.remove(&user_id);
                changed = true;
            }
        }

        if changed {
            self.publish_game_watch(game_id);
            self.publish_lobby_watch();
        }
    }

    pub(crate) fn apply_action(
        &mut self,
        game_id: i64,
        user: AuthUser,
        expected_version: i64,
        action: Action,
    ) -> Result<(GameStateResponse, Option<CompletedGameRecord>), RuntimeError> {
        let mut lobby_changed = false;
        {
            let game = self.games.get_mut(&game_id).ok_or(RuntimeError::NotFound)?;
            if game.status != STATUS_STARTED {
                return Err(RuntimeError::Conflict);
            }
            let seat = game
                .seats
                .iter()
                .find(|seat| seat.user_id == user.id)
                .map(|seat| seat.seat)
                .ok_or(RuntimeError::Forbidden)?;
            if game.version != expected_version {
                return Err(RuntimeError::Conflict);
            }

            let scrap_len_before = game.engine.scrap.len();
            let phase_before = game.engine.phase.clone();
            game.engine
                .apply(seat, action.clone())
                .map_err(|_| RuntimeError::BadRequest)?;
            append_action(&mut game.tokenlog_full, seat, &action)
                .map_err(|_| RuntimeError::BadRequest)?;
            game.last_event = Some(build_last_event(seat, &action, &phase_before));
            game.version += 1;

            if game.engine.winner.is_some() && game.status != STATUS_FINISHED {
                game.status = STATUS_FINISHED;
                let finished_at = Utc::now();
                game.finished_at = Some(finished_at);
                lobby_changed = true;
            }
            if game.engine.scrap.len() > scrap_len_before && game.scrap_straightened {
                game.scrap_straightened = false;
            }
        }

        self.publish_game_watch(game_id);
        if lobby_changed {
            self.publish_lobby_watch();
        }

        let state = self.build_state_response_for_user(game_id, &user, false)?;
        let completed_record = {
            let game = self.games.get(&game_id).ok_or(RuntimeError::NotFound)?;
            if game.status == STATUS_FINISHED {
                game.finished_at
                    .and_then(|finished_at| Self::build_completed_record(game, finished_at))
            } else {
                None
            }
        };
        Ok((state, completed_record))
    }

    fn user_ids_by_seat(game: &GameEntry) -> Option<[i64; 3]> {
        let mut user_ids: [Option<i64>; 3] = [None, None, None];
        for seat in &game.seats {
            let idx = seat.seat as usize;
            if idx < 3 {
                user_ids[idx] = Some(seat.user_id);
            }
        }
        Some([user_ids[0]?, user_ids[1]?, user_ids[2]?])
    }

    fn build_completed_record(
        game: &GameEntry,
        finished_at: DateTime<Utc>,
    ) -> Option<CompletedGameRecord> {
        let started_at = game.started_at?;
        let [p0_user_id, p1_user_id, p2_user_id] = Self::user_ids_by_seat(game)?;
        Some(CompletedGameRecord {
            rust_game_id: game.id,
            tokenlog: game.tokenlog_full.clone(),
            p0_user_id,
            p1_user_id,
            p2_user_id,
            started_at,
            finished_at,
        })
    }

    pub(crate) fn toggle_scrap_straighten(
        &mut self,
        game_id: i64,
        user: AuthUser,
    ) -> Result<(), RuntimeError> {
        {
            let game = self.games.get_mut(&game_id).ok_or(RuntimeError::NotFound)?;
            let _seat = game
                .seats
                .iter()
                .find(|seat| seat.user_id == user.id)
                .map(|seat| seat.seat)
                .ok_or(RuntimeError::Forbidden)?;

            game.scrap_straightened = !game.scrap_straightened;
        }

        self.publish_game_watch(game_id);
        Ok(())
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
