use crate::api::handlers::{
    GameStateResponse, LobbyListsResponse, LobbySeatView, LobbySummary, LobbyView,
    SpectatableGameSummary,
};
use crate::auth::AuthUser;
use crate::persistence::CompletedGameRecord;
use crate::state::{GameUpdate, LobbyListUpdate, ScrapStraightenUpdate};
use crate::store::{STATUS_FINISHED, STATUS_LOBBY, STATUS_STARTED};
use crate::view::history::build_history_log_for_viewer;
use crate::view::response::{
    build_last_event, build_spectator_view, format_action, normal_lobby_name,
    redact_tokenlog_for_client, usernames_from_seats,
};
use axum::http::StatusCode;
use chrono::{DateTime, Utc};
use cutthroat_engine::{
    Action, CutthroatState, LastEventView, Phase, Seat, Winner, append_action, encode_header,
};
use rand::seq::SliceRandom;
use std::collections::HashMap;
use tokio::sync::broadcast;

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

#[derive(Debug)]
pub(crate) enum StoreError {
    NotFound,
    Forbidden,
    Conflict,
    BadRequest,
}

impl StoreError {
    pub(crate) fn status_code(&self) -> StatusCode {
        match self {
            StoreError::NotFound => StatusCode::NOT_FOUND,
            StoreError::Forbidden => StatusCode::FORBIDDEN,
            StoreError::Conflict => StatusCode::CONFLICT,
            StoreError::BadRequest => StatusCode::BAD_REQUEST,
        }
    }

    pub(crate) fn code(&self) -> u16 {
        self.status_code().as_u16()
    }

    pub(crate) fn message(&self) -> String {
        match self {
            StoreError::NotFound => "not found".to_string(),
            StoreError::Forbidden => "forbidden".to_string(),
            StoreError::Conflict => "conflict".to_string(),
            StoreError::BadRequest => "bad request".to_string(),
        }
    }
}

pub(crate) struct Store {
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

    pub(crate) fn new(
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

    pub(crate) fn lobby_list_for_user(&self, user_id: Option<i64>) -> LobbyListsResponse {
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
            .map(|game| SpectatableGameSummary {
                id: game.id,
                name: game.name.clone(),
                seat_count: game.seats.len(),
                status: game.status,
                spectating_usernames: Self::active_spectator_usernames(game),
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
        self.broadcast_lobbies();
        id
    }

    pub(crate) fn rematch_game(&mut self, game_id: i64, user: AuthUser) -> Result<i64, StoreError> {
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

    pub(crate) fn join_game(&mut self, game_id: i64, user: AuthUser) -> Result<Seat, StoreError> {
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

    pub(crate) fn leave_game(&mut self, game_id: i64, user: AuthUser) -> Result<(), StoreError> {
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

    pub(crate) fn set_ready(
        &mut self,
        game_id: i64,
        user: AuthUser,
        ready: bool,
    ) -> Result<(), StoreError> {
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

    pub(crate) fn start_game(&mut self, game_id: i64) -> Result<(), StoreError> {
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

    pub(crate) fn validate_viewer(
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

    pub(crate) fn build_state_response_for_user(
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

    pub(crate) fn spectator_connected(
        &mut self,
        game_id: i64,
        user: AuthUser,
    ) -> Result<(), StoreError> {
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

    pub(crate) fn spectator_disconnected(&mut self, game_id: i64, user_id: i64) {
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

    pub(crate) fn apply_action(
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

    pub(crate) fn toggle_scrap_straighten(
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
