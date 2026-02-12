use crate::api::handlers::{GameStateResponse, LobbySeatView, LobbyView};
use crate::auth::AuthUser;
use crate::game_runtime::commands::GameCommand;
#[cfg(feature = "e2e-seed")]
use crate::game_runtime::commands::{SeedGameFromTranscriptInput, SeedGameInput, SeedGameResult};
use crate::game_runtime::types::{
    GameAudience, GameEntry, GameHandle, GameStreamSubscription, GlobalRuntimeState, RuntimeError,
    SeatEntry, active_spectator_usernames,
};
use crate::game_runtime::{STATUS_FINISHED, STATUS_LOBBY, STATUS_STARTED};
use crate::persistence::{CompletedGameRecord, PersistenceWriteMessage};
use crate::view::history::{build_history_log_for_viewer, build_history_log_for_viewer_with_limit};
use crate::view::response::{
    build_last_event, build_spectator_view, legal_action_tokens_for_seat, normal_lobby_name,
    redact_tokenlog_for_client, serialize_tokenlog,
};
use chrono::{DateTime, Utc};
#[cfg(feature = "e2e-seed")]
use cutthroat_engine::parse_tokenlog;
use cutthroat_engine::{
    CutthroatState, Phase, Seat, TokenLog, parse_action_token_stream_for_state, parse_token_slice,
    replay_tokenlog,
};
use rand::seq::SliceRandom;
use std::collections::HashMap;
#[cfg(feature = "e2e-seed")]
use std::collections::{BTreeMap, HashSet};
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc, watch};
use tracing::error;

const GAME_COMMAND_BUFFER: usize = 256;
const FINISHED_CLEANUP_GRACE_SECONDS: i64 = 5;

struct GameWatchSet {
    seat_tx: [watch::Sender<Arc<GameStateResponse>>; 3],
    spectator_tx: watch::Sender<Arc<GameStateResponse>>,
}

struct GameActor {
    runtime: Arc<RwLock<GlobalRuntimeState>>,
    persistence_tx: mpsc::Sender<PersistenceWriteMessage>,
    game: GameEntry,
    streams: GameWatchSet,
    seat_connections: [usize; 3],
}

struct SubscribeOutcome {
    subscription: GameStreamSubscription,
    game_changed: bool,
    lobby_changed: bool,
    seat_connection_changed: bool,
}

struct DisconnectOutcome {
    game_changed: bool,
    lobby_changed: bool,
    seat_connection_changed: bool,
}

struct ApplyActionOutcome {
    state: GameStateResponse,
    completed_record: Option<CompletedGameRecord>,
    lobby_changed: bool,
}

enum LeaveOutcome {
    KeepGame,
    RemoveGame,
}

pub(crate) async fn game_sender(
    runtime: &Arc<RwLock<GlobalRuntimeState>>,
    game_id: i64,
) -> Option<mpsc::Sender<GameCommand>> {
    let guard = runtime.read().await;
    guard.games.get(&game_id).map(|handle| handle.tx.clone())
}

pub(crate) async fn create_game_for_user(
    runtime: Arc<RwLock<GlobalRuntimeState>>,
    persistence_tx: mpsc::Sender<PersistenceWriteMessage>,
    user: AuthUser,
) -> i64 {
    let id = {
        let mut guard = runtime.write().await;
        let id = guard.next_id;
        guard.next_id += 1;
        id
    };

    let game = new_lobby_game(id, user);
    let _ = spawn_game_actor(runtime, persistence_tx, game).await;
    id
}

pub(crate) async fn create_rematch_for_user(
    runtime: Arc<RwLock<GlobalRuntimeState>>,
    persistence_tx: mpsc::Sender<PersistenceWriteMessage>,
    source_game_id: i64,
    user: AuthUser,
) -> Result<i64, RuntimeError> {
    let source_meta = {
        let guard = runtime.read().await;
        guard
            .game_meta
            .get(&source_game_id)
            .cloned()
            .ok_or(RuntimeError::NotFound)?
    };

    if source_meta.status != STATUS_FINISHED {
        return Err(RuntimeError::Conflict);
    }
    if !source_meta.seats.iter().any(|seat| seat.user_id == user.id) {
        return Err(RuntimeError::Forbidden);
    }

    {
        let guard = runtime.read().await;
        if let Some(existing_id) = guard.rematches.get(&source_game_id).copied()
            && guard
                .game_meta
                .get(&existing_id)
                .map(|meta| meta.status == STATUS_LOBBY)
                .unwrap_or(false)
        {
            if persistence_tx
                .try_send(PersistenceWriteMessage::LinkRematch {
                    source_game_id,
                    next_game_id: existing_id,
                })
                .is_err()
            {
                error!("persistence worker channel full/closed; dropping rematch link write");
            }
            return Ok(existing_id);
        }
    }

    let mut seats = source_meta.seats.clone();
    seats.sort_by_key(|seat| seat.seat);
    for seat in &mut seats {
        seat.ready = false;
    }

    let series_order = if source_meta.series_player_order.is_empty() {
        seats.iter().map(|seat| seat.user_id).collect::<Vec<i64>>()
    } else {
        source_meta.series_player_order.clone()
    };

    let (id, rematch_name) = {
        let mut guard = runtime.write().await;
        if let Some(existing_id) = guard.rematches.get(&source_game_id).copied()
            && guard
                .game_meta
                .get(&existing_id)
                .map(|meta| meta.status == STATUS_LOBBY)
                .unwrap_or(false)
        {
            if persistence_tx
                .try_send(PersistenceWriteMessage::LinkRematch {
                    source_game_id,
                    next_game_id: existing_id,
                })
                .is_err()
            {
                error!("persistence worker channel full/closed; dropping rematch link write");
            }
            return Ok(existing_id);
        }

        let id = guard.next_id;
        guard.next_id += 1;
        let rematch_name = guard.rematch_series_name(source_game_id, &series_order);
        (id, rematch_name)
    };

    let rematch_dealer = (source_meta.dealer + 1) % 3;

    let mut deck = cutthroat_engine::full_deck_with_jokers();
    deck.shuffle(&mut rand::thread_rng());
    let transcript = TokenLog {
        dealer: rematch_dealer,
        deck: deck.clone(),
        actions: Vec::new(),
    };
    let engine = CutthroatState::new_with_deck(rematch_dealer, deck);

    let rematch = GameEntry {
        id,
        name: rematch_name,
        status: STATUS_LOBBY,
        is_rematch_lobby: true,
        rematch_from_game_id: Some(source_game_id),
        series_anchor_game_id: source_meta.series_anchor_game_id,
        series_player_order: series_order,
        seats,
        transcript,
        last_event: None,
        scrap_straightened: false,
        started_at: Utc::now(),
        finished_at: Utc::now(),
        active_spectators: HashMap::new(),
        version: 0,
        engine,
    };

    let _ = spawn_game_actor_internal(
        runtime.clone(),
        persistence_tx.clone(),
        rematch,
        Some(source_game_id),
    )
    .await;
    if persistence_tx
        .try_send(PersistenceWriteMessage::LinkRematch {
            source_game_id,
            next_game_id: id,
        })
        .is_err()
    {
        error!("persistence worker channel full/closed; dropping rematch link write");
    }
    Ok(id)
}

pub(crate) async fn spawn_game_actor(
    runtime: Arc<RwLock<GlobalRuntimeState>>,
    persistence_tx: mpsc::Sender<PersistenceWriteMessage>,
    game: GameEntry,
) -> mpsc::Sender<GameCommand> {
    spawn_game_actor_internal(runtime, persistence_tx, game, None).await
}

async fn spawn_game_actor_internal(
    runtime: Arc<RwLock<GlobalRuntimeState>>,
    persistence_tx: mpsc::Sender<PersistenceWriteMessage>,
    game: GameEntry,
    rematch_source_game_id: Option<i64>,
) -> mpsc::Sender<GameCommand> {
    let (tx, rx) = mpsc::channel(GAME_COMMAND_BUFFER);

    {
        let mut guard = runtime.write().await;
        guard.games.insert(game.id, GameHandle { tx: tx.clone() });
        if let Some(source_id) = rematch_source_game_id {
            guard.rematches.insert(source_id, game.id);
        }
        guard.upsert_game_state(&game);
        guard.publish_lobby_watch();
    }

    let actor = GameActor::new(runtime, persistence_tx, game);
    tokio::spawn(actor.run(rx));
    tx
}

impl GameActor {
    fn new(
        runtime: Arc<RwLock<GlobalRuntimeState>>,
        persistence_tx: mpsc::Sender<PersistenceWriteMessage>,
        game: GameEntry,
    ) -> Self {
        let seat0 = Arc::new(
            build_state_response(&game, 0).unwrap_or_else(|_| fallback_state_response(&game, 0)),
        );
        let seat1 = Arc::new(
            build_state_response(&game, 1).unwrap_or_else(|_| fallback_state_response(&game, 1)),
        );
        let seat2 = Arc::new(
            build_state_response(&game, 2).unwrap_or_else(|_| fallback_state_response(&game, 2)),
        );
        let spectator = Arc::new(build_spectator_state_response(&game));

        let (seat0_tx, _seat0_rx) = watch::channel(seat0);
        let (seat1_tx, _seat1_rx) = watch::channel(seat1);
        let (seat2_tx, _seat2_rx) = watch::channel(seat2);
        let (spectator_tx, _spectator_rx) = watch::channel(spectator);

        Self {
            runtime,
            persistence_tx,
            game,
            streams: GameWatchSet {
                seat_tx: [seat0_tx, seat1_tx, seat2_tx],
                spectator_tx,
            },
            seat_connections: [0, 0, 0],
        }
    }

    async fn run(mut self, mut rx: mpsc::Receiver<GameCommand>) {
        self.publish_game_watch();

        while let Some(cmd) = rx.recv().await {
            let mut publish_game = false;
            let mut publish_lobby = false;
            let mut notify_source_rematch_started = false;
            let mut maybe_cleanup_unstarted_rematch = false;
            let mut stop = false;

            match cmd {
                GameCommand::JoinGame { user, respond } => {
                    let result = self.join_game(user);
                    if result.is_ok() {
                        publish_game = true;
                        publish_lobby = true;
                    }
                    let _ = respond.send(result);
                }
                GameCommand::LeaveGame { user, respond } => match self.leave_game(user) {
                    Ok(LeaveOutcome::KeepGame) => {
                        publish_game = true;
                        publish_lobby = true;
                        let _ = respond.send(Ok(()));
                    }
                    Ok(LeaveOutcome::RemoveGame) => {
                        let _ = respond.send(Ok(()));
                        self.deregister_self().await;
                        stop = true;
                    }
                    Err(err) => {
                        let _ = respond.send(Err(err));
                    }
                },
                GameCommand::SetReady {
                    user,
                    ready,
                    respond,
                } => {
                    let result = self.set_ready(user, ready);
                    match result {
                        Ok(started_transition) => {
                            publish_game = true;
                            publish_lobby = true;
                            notify_source_rematch_started = started_transition;
                            let _ = respond.send(Ok(()));
                        }
                        Err(err) => {
                            let _ = respond.send(Err(err));
                        }
                    }
                }
                GameCommand::StartGame { user, respond } => {
                    let result = self.start_game(user);
                    match result {
                        Ok(started_transition) => {
                            publish_game = true;
                            publish_lobby = true;
                            notify_source_rematch_started = started_transition;
                            let _ = respond.send(Ok(()));
                        }
                        Err(err) => {
                            let _ = respond.send(Err(err));
                        }
                    }
                }
                GameCommand::GetState {
                    user,
                    spectate_intent,
                    respond,
                } => {
                    let result = self.build_state_response_for_user(&user, spectate_intent);
                    let _ = respond.send(result);
                }
                GameCommand::GetSpectateReplayState {
                    user,
                    game_state_index,
                    respond,
                } => {
                    let result =
                        self.build_spectator_replay_state_for_user(&user, game_state_index);
                    let _ = respond.send(result);
                }
                GameCommand::SubscribeGameStream {
                    user,
                    spectate_intent,
                    respond,
                } => {
                    let result = self.subscribe_game_stream(user, spectate_intent);
                    match result {
                        Ok(outcome) => {
                            publish_game |= outcome.game_changed;
                            publish_lobby |= outcome.lobby_changed;
                            maybe_cleanup_unstarted_rematch |= outcome.seat_connection_changed;
                            let _ = respond.send(Ok(outcome.subscription));
                        }
                        Err(err) => {
                            let _ = respond.send(Err(err));
                        }
                    }
                }
                GameCommand::SocketDisconnected { user_id, audience } => {
                    let outcome = self.socket_disconnected(user_id, audience);
                    publish_game |= outcome.game_changed;
                    publish_lobby |= outcome.lobby_changed;
                    maybe_cleanup_unstarted_rematch |= outcome.seat_connection_changed;
                }
                GameCommand::SyncRematchPresenceFromSource {
                    disconnected_user_ids,
                } => {
                    if self.sync_rematch_presence_from_source(&disconnected_user_ids) {
                        publish_game = true;
                        publish_lobby = true;
                    }
                }
                GameCommand::ApplyAction {
                    user,
                    expected_version,
                    action_tokens,
                    respond,
                } => {
                    let result = self.apply_action(user, expected_version, action_tokens);
                    match result {
                        Ok(outcome) => {
                            if let Some(record) = outcome.completed_record
                                && self
                                    .persistence_tx
                                    .try_send(PersistenceWriteMessage::CompletedGame(record))
                                    .is_err()
                            {
                                error!(
                                    "persistence worker channel full/closed; dropping completed game record"
                                );
                            }
                            publish_game = true;
                            publish_lobby |= outcome.lobby_changed;
                            let _ = respond.send(Ok(outcome.state));
                        }
                        Err(err) => {
                            let _ = respond.send(Err(err));
                        }
                    }
                }
                GameCommand::ToggleScrapStraighten { user, respond } => {
                    let result = self.toggle_scrap_straighten(user);
                    if result.is_ok() {
                        publish_game = true;
                    }
                    let _ = respond.send(result);
                }
                GameCommand::EvaluateCleanup => {}
                GameCommand::Shutdown => {
                    stop = true;
                }
            }

            if publish_game {
                self.publish_game_watch();
            }
            if publish_lobby {
                self.publish_global_state().await;
            }
            if notify_source_rematch_started {
                self.notify_source_rematch_started().await;
            }
            if maybe_cleanup_unstarted_rematch {
                self.sync_unstarted_rematch_presence_from_source_disconnects()
                    .await;
            }
            if stop {
                break;
            }
            if self.should_cleanup().await {
                self.sync_unstarted_rematch_presence_from_source_disconnects()
                    .await;
                self.deregister_self().await;
                break;
            }
        }
    }

    fn join_game(&mut self, user: AuthUser) -> Result<Seat, RuntimeError> {
        if self.game.status != STATUS_LOBBY {
            return Err(RuntimeError::Conflict);
        }

        if let Some(existing) = self.game.seats.iter().find(|seat| seat.user_id == user.id) {
            return Ok(existing.seat);
        }

        if self.game.is_rematch_lobby {
            let seat_index = self
                .game
                .series_player_order
                .iter()
                .position(|user_id| *user_id == user.id)
                .ok_or(RuntimeError::Forbidden)? as Seat;

            if let Some(existing) = self.game.seats.iter().find(|seat| seat.seat == seat_index) {
                if existing.user_id == user.id {
                    return Ok(existing.seat);
                }
                return Err(RuntimeError::Conflict);
            }

            self.game.seats.push(SeatEntry {
                seat: seat_index,
                user_id: user.id,
                username: user.username,
                ready: false,
            });
            return Ok(seat_index);
        }

        if self.game.seats.len() >= 3 {
            return Err(RuntimeError::Conflict);
        }

        let mut occupied = [false; 3];
        for seat in &self.game.seats {
            occupied[seat.seat as usize] = true;
        }

        let seat_index = occupied
            .iter()
            .position(|occupied| !*occupied)
            .ok_or(RuntimeError::Conflict)?;

        self.game.seats.push(SeatEntry {
            seat: seat_index as Seat,
            user_id: user.id,
            username: user.username,
            ready: false,
        });

        if !self.game.is_rematch_lobby {
            self.game.name = normal_lobby_name(&self.game.seats);
        }

        Ok(seat_index as Seat)
    }

    fn leave_game(&mut self, user: AuthUser) -> Result<LeaveOutcome, RuntimeError> {
        if self.game.status != STATUS_LOBBY {
            return Err(RuntimeError::Conflict);
        }

        let idx = self
            .game
            .seats
            .iter()
            .position(|seat| seat.user_id == user.id)
            .ok_or(RuntimeError::Forbidden)?;

        self.game.seats.remove(idx);

        if self.game.seats.is_empty() {
            return Ok(LeaveOutcome::RemoveGame);
        }

        if !self.game.is_rematch_lobby {
            self.game.name = normal_lobby_name(&self.game.seats);
        }

        Ok(LeaveOutcome::KeepGame)
    }

    fn set_ready(&mut self, user: AuthUser, ready: bool) -> Result<bool, RuntimeError> {
        if self.game.status != STATUS_LOBBY {
            return Err(RuntimeError::Conflict);
        }

        let seat = self
            .game
            .seats
            .iter_mut()
            .find(|seat| seat.user_id == user.id)
            .ok_or(RuntimeError::Forbidden)?;
        seat.ready = ready;

        let mut started_transition = false;
        if self.game.seats.len() == 3 && self.game.seats.iter().all(|seat| seat.ready) {
            self.game.status = STATUS_STARTED;
            self.game.started_at = Utc::now();
            started_transition = true;
        }

        Ok(started_transition)
    }

    fn start_game(&mut self, user: AuthUser) -> Result<bool, RuntimeError> {
        if !self.game.seats.iter().any(|seat| seat.user_id == user.id) {
            return Err(RuntimeError::Forbidden);
        }
        if self.game.status != STATUS_LOBBY {
            return Err(RuntimeError::Conflict);
        }
        if self.game.seats.len() != 3 || !self.game.seats.iter().all(|seat| seat.ready) {
            return Err(RuntimeError::Conflict);
        }

        self.game.status = STATUS_STARTED;
        self.game.started_at = Utc::now();
        Ok(true)
    }

    fn validate_viewer(&self, user: &AuthUser, spectate_intent: bool) -> Result<(), RuntimeError> {
        let viewer_is_seated = self.game.seats.iter().any(|seat| seat.user_id == user.id);

        if spectate_intent {
            if viewer_is_seated && self.game.status != STATUS_FINISHED {
                return Err(RuntimeError::Conflict);
            }
            if self.game.status != STATUS_STARTED && self.game.status != STATUS_FINISHED {
                return Err(RuntimeError::Conflict);
            }
            return Ok(());
        }

        if viewer_is_seated {
            return Ok(());
        }

        if self.game.status == STATUS_LOBBY {
            return Err(RuntimeError::Conflict);
        }

        Ok(())
    }

    fn subscribe_game_stream(
        &mut self,
        user: AuthUser,
        spectate_intent: bool,
    ) -> Result<SubscribeOutcome, RuntimeError> {
        self.validate_viewer(&user, spectate_intent)?;

        let maybe_seat = self
            .game
            .seats
            .iter()
            .find(|seat| seat.user_id == user.id)
            .map(|seat| seat.seat);

        let audience = if spectate_intent || maybe_seat.is_none() {
            GameAudience::Spectator
        } else {
            GameAudience::Seat(maybe_seat.unwrap_or(0))
        };

        let mut game_changed = false;
        let mut lobby_changed = false;
        let mut seat_connection_changed = false;

        let rx = match audience {
            GameAudience::Spectator => {
                let entry = self
                    .game
                    .active_spectators
                    .entry(user.id)
                    .or_insert((user.username, 0));
                entry.1 += 1;
                game_changed = true;
                lobby_changed = true;
                self.streams.spectator_tx.subscribe()
            }
            GameAudience::Seat(seat) => {
                let idx = seat as usize;
                if idx >= 3 {
                    return Err(RuntimeError::BadRequest);
                }
                self.seat_connections[idx] = self.seat_connections[idx].saturating_add(1);
                seat_connection_changed = true;
                self.streams.seat_tx[idx].subscribe()
            }
        };

        Ok(SubscribeOutcome {
            subscription: GameStreamSubscription { audience, rx },
            game_changed,
            lobby_changed,
            seat_connection_changed,
        })
    }

    fn socket_disconnected(&mut self, user_id: i64, audience: GameAudience) -> DisconnectOutcome {
        match audience {
            GameAudience::Seat(seat) => {
                let idx = seat as usize;
                if idx < 3 && self.seat_connections[idx] > 0 {
                    self.seat_connections[idx] -= 1;
                }
                DisconnectOutcome {
                    game_changed: false,
                    lobby_changed: false,
                    seat_connection_changed: true,
                }
            }
            GameAudience::Spectator => {
                let mut changed = false;
                if let Some((_, count)) = self.game.active_spectators.get_mut(&user_id) {
                    if *count > 1 {
                        *count -= 1;
                    } else {
                        self.game.active_spectators.remove(&user_id);
                    }
                    changed = true;
                }

                DisconnectOutcome {
                    game_changed: changed,
                    lobby_changed: changed,
                    seat_connection_changed: false,
                }
            }
        }
    }

    fn sync_rematch_presence_from_source(&mut self, disconnected_user_ids: &[i64]) -> bool {
        if !self.game.is_rematch_lobby || self.game.status != STATUS_LOBBY {
            return false;
        }
        if disconnected_user_ids.is_empty() {
            return false;
        }

        let disconnected: std::collections::HashSet<i64> =
            disconnected_user_ids.iter().copied().collect();
        let original_len = self.game.seats.len();
        self.game.seats.retain(|seat| {
            let seat_idx = seat.seat as usize;
            let active_rematch_connection =
                seat_idx < self.seat_connections.len() && self.seat_connections[seat_idx] > 0;
            if active_rematch_connection {
                return true;
            }
            !disconnected.contains(&seat.user_id)
        });

        original_len != self.game.seats.len()
    }

    fn build_state_response_for_user(
        &self,
        user: &AuthUser,
        spectate_intent: bool,
    ) -> Result<GameStateResponse, RuntimeError> {
        self.validate_viewer(user, spectate_intent)?;

        let maybe_seat = self
            .game
            .seats
            .iter()
            .find(|seat| seat.user_id == user.id)
            .map(|seat| seat.seat);

        let mut response = if spectate_intent || maybe_seat.is_none() {
            build_spectator_state_response(&self.game)
        } else {
            build_state_response(&self.game, maybe_seat.unwrap_or(0))?
        };
        response.has_active_seated_players = self.seat_connections.iter().any(|count| *count > 0);
        Ok(response)
    }

    fn build_spectator_replay_state_for_user(
        &self,
        user: &AuthUser,
        game_state_index: i64,
    ) -> Result<GameStateResponse, RuntimeError> {
        self.validate_viewer(user, true)?;

        if game_state_index < 0 {
            let mut response = build_spectator_state_response(&self.game);
            response.has_active_seated_players =
                self.seat_connections.iter().any(|count| *count > 0);
            return Ok(response);
        }

        if self.game.status != STATUS_FINISHED {
            let mut response = build_spectator_state_response(&self.game);
            response.has_active_seated_players =
                self.seat_connections.iter().any(|count| *count > 0);
            return Ok(response);
        }

        let replay_index =
            usize::try_from(game_state_index).map_err(|_| RuntimeError::BadRequest)?;
        if replay_index > self.game.transcript.actions.len() {
            return Err(RuntimeError::NotFound);
        }

        let mut truncated = self.game.transcript.clone();
        truncated.actions.truncate(replay_index);
        let replayed = replay_tokenlog(&truncated).map_err(|_| RuntimeError::BadRequest)?;

        let mut replay_game = self.game.clone();
        replay_game.engine = replayed;
        replay_game.version = replay_index as i64;
        replay_game.last_event = None;
        replay_game.scrap_straightened = false;
        replay_game.status = if replay_index < self.game.transcript.actions.len() {
            STATUS_STARTED
        } else {
            STATUS_FINISHED
        };

        let mut response = build_spectator_state_response(&replay_game);
        response.tokenlog = redact_tokenlog_for_client(&self.game.transcript, None);
        response.replay_total_states = replay_total_states(&self.game);
        response.log_tail =
            build_history_log_for_viewer_with_limit(&replay_game, 0, Some(replay_index));
        response.has_active_seated_players = self.seat_connections.iter().any(|count| *count > 0);
        Ok(response)
    }

    fn apply_action(
        &mut self,
        user: AuthUser,
        expected_version: i64,
        action_tokens: String,
    ) -> Result<ApplyActionOutcome, RuntimeError> {
        if self.game.status != STATUS_STARTED {
            return Err(RuntimeError::Conflict);
        }

        let seat = self
            .game
            .seats
            .iter()
            .find(|seat| seat.user_id == user.id)
            .map(|seat| seat.seat)
            .ok_or(RuntimeError::Forbidden)?;

        if self.game.version != expected_version {
            return Err(RuntimeError::Conflict);
        }

        let mut tokens = parse_token_slice(&action_tokens).ok_or(RuntimeError::BadRequest)?;
        let (declared_seat, action) =
            parse_action_token_stream_for_state(&mut tokens, &self.game.engine)
                .map_err(|_| RuntimeError::BadRequest)?;
        if declared_seat != seat {
            return Err(RuntimeError::Forbidden);
        }

        let scrap_len_before = self.game.engine.scrap.len();
        let phase_before = self.game.engine.phase.clone();

        self.game
            .engine
            .apply(seat, action.clone())
            .map_err(|_| RuntimeError::BadRequest)?;
        self.game.transcript.actions.push((seat, action.clone()));
        self.game.last_event = Some(build_last_event(seat, &action, &phase_before));
        self.game.version = self.game.transcript.actions.len() as i64;

        let mut lobby_changed = false;
        if self.game.engine.winner.is_some() && self.game.status != STATUS_FINISHED {
            self.game.status = STATUS_FINISHED;
            self.game.finished_at = Utc::now();
            lobby_changed = true;
        }

        if self.game.engine.scrap.len() > scrap_len_before && self.game.scrap_straightened {
            self.game.scrap_straightened = false;
        }

        let state = self.build_state_response_for_user(&user, false)?;
        let completed_record = if self.game.status == STATUS_FINISHED {
            build_completed_record(&self.game, self.game.finished_at)
        } else {
            None
        };

        Ok(ApplyActionOutcome {
            state,
            completed_record,
            lobby_changed,
        })
    }

    fn toggle_scrap_straighten(&mut self, user: AuthUser) -> Result<(), RuntimeError> {
        let viewer_is_seated = self.game.seats.iter().any(|seat| seat.user_id == user.id);
        let viewer_is_active_spectator = self
            .game
            .active_spectators
            .get(&user.id)
            .map(|(_, count)| *count > 0)
            .unwrap_or(false);

        if !viewer_is_seated && !viewer_is_active_spectator {
            return Err(RuntimeError::Forbidden);
        }

        self.game.scrap_straightened = !self.game.scrap_straightened;
        Ok(())
    }

    fn publish_game_watch(&self) {
        let Ok(mut seat0) = build_state_response(&self.game, 0) else {
            return;
        };
        let Ok(mut seat1) = build_state_response(&self.game, 1) else {
            return;
        };
        let Ok(mut seat2) = build_state_response(&self.game, 2) else {
            return;
        };
        let mut spectator = build_spectator_state_response(&self.game);
        let has_active_seated_players = self.seat_connections.iter().any(|count| *count > 0);
        seat0.has_active_seated_players = has_active_seated_players;
        seat1.has_active_seated_players = has_active_seated_players;
        seat2.has_active_seated_players = has_active_seated_players;
        spectator.has_active_seated_players = has_active_seated_players;

        self.streams.seat_tx[0].send_replace(Arc::new(seat0));
        self.streams.seat_tx[1].send_replace(Arc::new(seat1));
        self.streams.seat_tx[2].send_replace(Arc::new(seat2));
        self.streams.spectator_tx.send_replace(Arc::new(spectator));
    }

    async fn publish_global_state(&self) {
        let mut guard = self.runtime.write().await;
        if !guard.games.contains_key(&self.game.id) {
            return;
        }
        guard.upsert_game_state(&self.game);
        guard.publish_lobby_watch();
    }

    async fn notify_source_rematch_started(&self) {
        let Some(source_id) = self.game.rematch_from_game_id else {
            return;
        };

        let Some(tx) = game_sender(&self.runtime, source_id).await else {
            return;
        };

        let _ = tx.send(GameCommand::EvaluateCleanup).await;
    }

    async fn sync_unstarted_rematch_presence_from_source_disconnects(&self) {
        if self.game.status != STATUS_FINISHED {
            return;
        }
        let disconnected_user_ids: Vec<i64> = self
            .game
            .seats
            .iter()
            .filter(|seat| {
                let idx = seat.seat as usize;
                idx < self.seat_connections.len() && self.seat_connections[idx] == 0
            })
            .map(|seat| seat.user_id)
            .collect();
        if disconnected_user_ids.is_empty() {
            return;
        }

        let rematch_tx = {
            let guard = self.runtime.read().await;
            let Some(rematch_id) = guard.rematches.get(&self.game.id).copied() else {
                return;
            };

            let rematch_is_lobby = guard
                .game_meta
                .get(&rematch_id)
                .map(|meta| meta.status == STATUS_LOBBY)
                .unwrap_or(false);
            if !rematch_is_lobby {
                return;
            }

            guard.games.get(&rematch_id).map(|handle| handle.tx.clone())
        };

        if let Some(tx) = rematch_tx {
            let _ = tx
                .send(GameCommand::SyncRematchPresenceFromSource {
                    disconnected_user_ids,
                })
                .await;
        }
    }

    async fn should_cleanup(&self) -> bool {
        if self.game.status != STATUS_FINISHED {
            return false;
        }
        if self.persistence_tx.is_closed() {
            return false;
        }
        if (Utc::now() - self.game.finished_at).num_seconds() < FINISHED_CLEANUP_GRACE_SECONDS {
            return false;
        }

        let has_active_spectators = self
            .game
            .active_spectators
            .values()
            .any(|(_, count)| *count > 0);
        if has_active_spectators {
            return false;
        }

        let all_players_disconnected = self.seat_connections.iter().all(|count| *count == 0);

        let rematch_started = {
            let guard = self.runtime.read().await;
            guard
                .rematches
                .get(&self.game.id)
                .and_then(|rematch_id| guard.game_meta.get(rematch_id))
                .map(|meta| meta.status == STATUS_STARTED)
                .unwrap_or(false)
        };

        rematch_started || all_players_disconnected
    }

    async fn deregister_self(&self) {
        let mut guard = self.runtime.write().await;
        guard.remove_game(self.game.id);
        guard.publish_lobby_watch();
    }
}

fn action_seat_for_phase(game: &GameEntry) -> Seat {
    match &game.engine.phase {
        Phase::Countering(counter) => counter.next_seat,
        Phase::ResolvingThree { seat, .. }
        | Phase::ResolvingFour { seat, .. }
        | Phase::ResolvingFive { seat, .. }
        | Phase::ResolvingSeven { seat, .. } => *seat,
        _ => game.engine.turn,
    }
}

fn replay_total_states(game: &GameEntry) -> i64 {
    game.transcript.actions.len() as i64 + 1
}

fn build_spectator_state_response(game: &GameEntry) -> GameStateResponse {
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
    let action_seat = action_seat_for_phase(game);
    let legal_actions = if game.status == STATUS_STARTED {
        legal_action_tokens_for_seat(&game.engine, action_seat)
    } else {
        Vec::new()
    };
    let log_tail = build_history_log_for_viewer(game, 0);
    let tokenlog = redact_tokenlog_for_client(&game.transcript, None);

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
        replay_total_states: replay_total_states(game),
        is_spectator: true,
        spectating_usernames: active_spectator_usernames(game),
        scrap_straightened: game.scrap_straightened,
        archived: false,
        next_game_id: None,
        next_game_finished: false,
        has_active_seated_players: false,
    }
}

fn build_state_response(game: &GameEntry, seat: Seat) -> Result<GameStateResponse, RuntimeError> {
    let seat_idx = seat as usize;
    if seat_idx >= 3 {
        return Err(RuntimeError::BadRequest);
    }

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

    let legal_actions = if game.status == STATUS_STARTED {
        legal_action_tokens_for_seat(&game.engine, seat)
    } else {
        Vec::new()
    };

    let mut player_view = game.engine.public_view(seat);
    player_view.last_event = game.last_event.clone();
    let spectator_view = build_spectator_view(game);
    let log_tail = build_history_log_for_viewer(game, seat);
    let tokenlog = redact_tokenlog_for_client(&game.transcript, Some(seat));

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
        replay_total_states: replay_total_states(game),
        is_spectator: false,
        spectating_usernames: active_spectator_usernames(game),
        scrap_straightened: game.scrap_straightened,
        archived: false,
        next_game_id: None,
        next_game_finished: false,
        has_active_seated_players: false,
    })
}

fn fallback_state_response(game: &GameEntry, seat: Seat) -> GameStateResponse {
    let spectator = build_spectator_state_response(game);
    GameStateResponse {
        seat,
        is_spectator: false,
        ..spectator
    }
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
    let [p0_user_id, p1_user_id, p2_user_id] = user_ids_by_seat(game)?;
    Some(CompletedGameRecord {
        rust_game_id: game.id,
        next_rust_game_id: None,
        tokenlog: serialize_tokenlog(&game.transcript),
        p0_user_id,
        p1_user_id,
        p2_user_id,
        started_at: game.started_at,
        finished_at,
    })
}

fn new_lobby_game(id: i64, user: AuthUser) -> GameEntry {
    let mut deck = cutthroat_engine::full_deck_with_jokers();
    deck.shuffle(&mut rand::thread_rng());
    let transcript = TokenLog {
        dealer: 0,
        deck: deck.clone(),
        actions: Vec::new(),
    };
    let engine = CutthroatState::new_with_deck(0, deck);

    let seat = SeatEntry {
        seat: 0,
        user_id: user.id,
        username: user.username,
        ready: false,
    };

    let mut game = GameEntry {
        id,
        name: String::new(),
        status: STATUS_LOBBY,
        is_rematch_lobby: false,
        rematch_from_game_id: None,
        series_anchor_game_id: id,
        series_player_order: Vec::new(),
        seats: vec![seat],
        transcript,
        last_event: None,
        scrap_straightened: false,
        started_at: Utc::now(),
        finished_at: Utc::now(),
        active_spectators: HashMap::new(),
        version: 0,
        engine,
    };

    game.name = normal_lobby_name(&game.seats);
    game
}

#[cfg(feature = "e2e-seed")]
pub(crate) async fn seed_game_from_tokenlog(
    runtime: Arc<RwLock<GlobalRuntimeState>>,
    persistence_tx: mpsc::Sender<PersistenceWriteMessage>,
    seed: SeedGameInput,
) -> Result<SeedGameResult, RuntimeError> {
    let game = seeded_game_from_tokenlog(seed)?;
    let seat_user_ids = seed_result_user_ids(&game);
    let tokenlog = serialize_tokenlog(&game.transcript);

    let (created, old_tx) = {
        let mut guard = runtime.write().await;
        let created = !guard.games.contains_key(&game.id);
        let old_tx = guard.games.get(&game.id).map(|handle| handle.tx.clone());
        if game.id >= guard.next_id {
            guard.next_id = game.id + 1;
        }
        guard.remove_game(game.id);
        (created, old_tx)
    };

    if let Some(tx) = old_tx {
        let _ = tx.send(GameCommand::Shutdown).await;
    }

    let game_id = game.id;
    let version = game.version;
    let status = game.status;

    let _ = spawn_game_actor(runtime, persistence_tx, game).await;

    Ok(SeedGameResult {
        game_id,
        version,
        status,
        seat_user_ids,
        tokenlog,
        created,
        replaced_existing: !created,
    })
}

#[cfg(feature = "e2e-seed")]
pub(crate) async fn seed_game_from_transcript(
    runtime: Arc<RwLock<GlobalRuntimeState>>,
    persistence_tx: mpsc::Sender<PersistenceWriteMessage>,
    seed: SeedGameFromTranscriptInput,
) -> Result<SeedGameResult, RuntimeError> {
    if seed.game_id <= 0 {
        return Err(RuntimeError::BadRequest);
    }

    let mut deck = parse_token_slice(&seed.deck_tokens.join(" "))
        .ok_or(RuntimeError::BadRequest)?
        .into_iter()
        .map(|token| token.card().ok_or(RuntimeError::BadRequest))
        .collect::<Result<Vec<_>, _>>()?;
    if deck.is_empty() {
        return Err(RuntimeError::BadRequest);
    }

    let mut engine = CutthroatState::new_with_deck(seed.dealer_seat, deck.clone());
    let mut transcript = TokenLog {
        dealer: seed.dealer_seat,
        deck: std::mem::take(&mut deck),
        actions: Vec::new(),
    };

    for action_tokens in &seed.action_tokens {
        let mut tokens = parse_token_slice(action_tokens).ok_or(RuntimeError::BadRequest)?;
        let (seat, action) = parse_action_token_stream_for_state(&mut tokens, &engine)
            .map_err(|_| RuntimeError::BadRequest)?;
        engine
            .apply(seat, action.clone())
            .map_err(|_| RuntimeError::BadRequest)?;
        transcript.actions.push((seat, action));
    }

    let tokenlog = serialize_tokenlog(&transcript);
    seed_game_from_tokenlog(
        runtime,
        persistence_tx,
        SeedGameInput {
            game_id: seed.game_id,
            players: seed.players,
            dealer_seat: Some(seed.dealer_seat),
            tokenlog,
            status: seed.status,
            spectating_usernames: seed.spectating_usernames,
            name: seed.name,
        },
    )
    .await
}

#[cfg(feature = "e2e-seed")]
fn seeded_game_from_tokenlog(seed: SeedGameInput) -> Result<GameEntry, RuntimeError> {
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
    let action_count = parsed.actions.len() as i64;

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

    let mut seats = seed
        .players
        .into_iter()
        .map(|player| SeatEntry {
            seat: player.seat,
            user_id: player.user_id,
            username: player.username,
            ready: player.ready.unwrap_or(status != STATUS_LOBBY),
        })
        .collect::<Vec<_>>();
    seats.sort_by_key(|seat| seat.seat);

    let name = seed
        .name
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| normal_lobby_name(&seats));

    let active_spectators = seed
        .spectating_usernames
        .unwrap_or_default()
        .into_iter()
        .enumerate()
        .map(|(idx, username)| (-(idx as i64) - 1, (username, 1usize)))
        .collect::<HashMap<_, _>>();

    Ok(GameEntry {
        id: seed.game_id,
        name,
        status,
        is_rematch_lobby: false,
        rematch_from_game_id: None,
        series_anchor_game_id: seed.game_id,
        series_player_order: seats.iter().map(|seat| seat.user_id).collect(),
        seats,
        transcript: parsed,
        last_event: None,
        scrap_straightened: false,
        started_at: Utc::now(),
        finished_at: Utc::now(),
        active_spectators,
        version: action_count,
        engine,
    })
}

#[cfg(feature = "e2e-seed")]
fn seed_result_user_ids(game: &GameEntry) -> BTreeMap<String, i64> {
    let mut seat_user_ids = BTreeMap::new();
    for seat in &game.seats {
        seat_user_ids.insert(format!("{}", seat.seat), seat.user_id);
    }
    seat_user_ids
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{Duration, sleep};

    fn user(id: i64, username: &str) -> AuthUser {
        AuthUser {
            id,
            username: username.to_string(),
        }
    }

    fn create_started_game(
        runtime: Arc<RwLock<GlobalRuntimeState>>,
    ) -> (i64, AuthUser, AuthUser, AuthUser) {
        let p0 = user(1, "p0");
        let p1 = user(2, "p1");
        let p2 = user(3, "p2");

        let rt = tokio::runtime::Runtime::new().expect("runtime");
        let (persistence_tx, _persistence_rx) = mpsc::channel(8);

        let game_id = rt.block_on(async {
            create_game_for_user(runtime.clone(), persistence_tx.clone(), p0.clone()).await
        });

        let tx = rt
            .block_on(async { game_sender(&runtime, game_id).await })
            .expect("sender");

        let (join1_tx, join1_rx) = tokio::sync::oneshot::channel();
        rt.block_on(async {
            tx.send(GameCommand::JoinGame {
                user: p1.clone(),
                respond: join1_tx,
            })
            .await
            .expect("join1 send");
        });
        rt.block_on(async { join1_rx.await.expect("join1 recv") })
            .expect("join1 ok");

        let (join2_tx, join2_rx) = tokio::sync::oneshot::channel();
        rt.block_on(async {
            tx.send(GameCommand::JoinGame {
                user: p2.clone(),
                respond: join2_tx,
            })
            .await
            .expect("join2 send");
        });
        rt.block_on(async { join2_rx.await.expect("join2 recv") })
            .expect("join2 ok");

        for player in [p0.clone(), p1.clone(), p2.clone()] {
            let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();
            rt.block_on(async {
                tx.send(GameCommand::SetReady {
                    user: player,
                    ready: true,
                    respond: ready_tx,
                })
                .await
                .expect("ready send");
            });
            rt.block_on(async { ready_rx.await.expect("ready recv") })
                .expect("ready ok");
        }

        (game_id, p0, p1, p2)
    }

    fn finished_source_game(
        game_id: i64,
        p0: &AuthUser,
        p1: &AuthUser,
        p2: &AuthUser,
    ) -> GameEntry {
        let mut game = new_lobby_game(game_id, p0.clone());
        game.seats = vec![
            SeatEntry {
                seat: 0,
                user_id: p0.id,
                username: p0.username.clone(),
                ready: true,
            },
            SeatEntry {
                seat: 1,
                user_id: p1.id,
                username: p1.username.clone(),
                ready: true,
            },
            SeatEntry {
                seat: 2,
                user_id: p2.id,
                username: p2.username.clone(),
                ready: true,
            },
        ];
        game.name = normal_lobby_name(&game.seats);
        game.status = STATUS_FINISHED;
        game.series_anchor_game_id = game_id;
        game.series_player_order = vec![p0.id, p1.id, p2.id];
        game
    }

    async fn subscribe_as_seat(tx: &mpsc::Sender<GameCommand>, user: AuthUser) {
        let (resp_tx, resp_rx) = tokio::sync::oneshot::channel();
        tx.send(GameCommand::SubscribeGameStream {
            user,
            spectate_intent: false,
            respond: resp_tx,
        })
        .await
        .expect("subscribe seat send");
        resp_rx
            .await
            .expect("subscribe seat recv")
            .expect("subscribe seat ok");
    }

    async fn subscribe_as_spectator(tx: &mpsc::Sender<GameCommand>, user: AuthUser) {
        let (resp_tx, resp_rx) = tokio::sync::oneshot::channel();
        tx.send(GameCommand::SubscribeGameStream {
            user,
            spectate_intent: true,
            respond: resp_tx,
        })
        .await
        .expect("subscribe spectator send");
        resp_rx
            .await
            .expect("subscribe spectator recv")
            .expect("subscribe spectator ok");
    }

    async fn set_ready_ok(tx: &mpsc::Sender<GameCommand>, user: AuthUser, ready: bool) {
        let (resp_tx, resp_rx) = tokio::sync::oneshot::channel();
        tx.send(GameCommand::SetReady {
            user,
            ready,
            respond: resp_tx,
        })
        .await
        .expect("set ready send");
        resp_rx
            .await
            .expect("set ready recv")
            .expect("set ready ok");
    }

    async fn leave_game_ok(tx: &mpsc::Sender<GameCommand>, user: AuthUser) {
        let (resp_tx, resp_rx) = tokio::sync::oneshot::channel();
        tx.send(GameCommand::LeaveGame {
            user,
            respond: resp_tx,
        })
        .await
        .expect("leave game send");
        resp_rx
            .await
            .expect("leave game recv")
            .expect("leave game ok");
    }

    async fn join_game_result(
        tx: &mpsc::Sender<GameCommand>,
        user: AuthUser,
    ) -> Result<Seat, RuntimeError> {
        let (resp_tx, resp_rx) = tokio::sync::oneshot::channel();
        tx.send(GameCommand::JoinGame {
            user,
            respond: resp_tx,
        })
        .await
        .expect("join game send");
        resp_rx.await.expect("join game recv")
    }

    async fn send_disconnect(tx: &mpsc::Sender<GameCommand>, user_id: i64, audience: GameAudience) {
        tx.send(GameCommand::SocketDisconnected { user_id, audience })
            .await
            .expect("disconnect send");
    }

    async fn wait_until<F>(mut predicate: F)
    where
        F: FnMut() -> bool,
    {
        for _ in 0..40 {
            if predicate() {
                return;
            }
            sleep(Duration::from_millis(10)).await;
        }
        panic!("condition not met in time");
    }

    #[test]
    fn smoke_uses_new_runtime_path() {
        let runtime = Arc::new(RwLock::new(GlobalRuntimeState::new(1)));
        let (_game_id, _p0, _p1, _p2) = create_started_game(runtime);
    }

    #[test]
    fn started_non_rematch_game_exposes_no_rematch_source_in_spectatable_summary() {
        let runtime = Arc::new(RwLock::new(GlobalRuntimeState::new(90)));
        let (game_id, _p0, _p1, _p2) = create_started_game(runtime.clone());
        let rt = tokio::runtime::Runtime::new().expect("runtime");
        rt.block_on(async {
            let guard = runtime.read().await;
            let lobby_snapshot = guard.lobby_tx.borrow();
            let spectatable_game = lobby_snapshot
                .spectatable_games
                .iter()
                .find(|entry| entry.id == game_id)
                .expect("started game should be spectatable");
            assert_eq!(spectatable_game.rematch_from_game_id, None);
        });
    }

    #[tokio::test]
    async fn finished_game_is_not_cleaned_up_when_persistence_channel_is_closed() {
        let runtime = Arc::new(RwLock::new(GlobalRuntimeState::new(180)));
        let (persistence_tx, persistence_rx) = mpsc::channel(8);
        drop(persistence_rx);

        let p0 = user(81, "p0");
        let p1 = user(82, "p1");
        let p2 = user(83, "p2");
        let mut finished_game = finished_source_game(180, &p0, &p1, &p2);
        finished_game.finished_at =
            Utc::now() - chrono::Duration::seconds(FINISHED_CLEANUP_GRACE_SECONDS + 1);

        let actor = GameActor::new(runtime, persistence_tx, finished_game);
        assert!(!actor.should_cleanup().await);
    }

    #[tokio::test]
    async fn source_disconnect_clears_unstarted_rematch_active_presence_but_keeps_reservations() {
        let runtime = Arc::new(RwLock::new(GlobalRuntimeState::new(100)));
        let (persistence_tx, _persistence_rx) = mpsc::channel(8);
        let p0 = user(10, "p0");
        let p1 = user(11, "p1");
        let p2 = user(12, "p2");
        let source_id = 42;

        let source_tx = spawn_game_actor(
            runtime.clone(),
            persistence_tx.clone(),
            finished_source_game(source_id, &p0, &p1, &p2),
        )
        .await;

        subscribe_as_seat(&source_tx, p0.clone()).await;
        subscribe_as_seat(&source_tx, p1.clone()).await;
        subscribe_as_seat(&source_tx, p2.clone()).await;

        let rematch_id = create_rematch_for_user(
            runtime.clone(),
            persistence_tx.clone(),
            source_id,
            p0.clone(),
        )
        .await
        .expect("create rematch");
        {
            let guard = runtime.read().await;
            assert_eq!(guard.rematches.get(&source_id), Some(&rematch_id));
            assert_eq!(
                guard.game_meta.get(&rematch_id).map(|meta| meta.status),
                Some(STATUS_LOBBY)
            );
            assert!(guard.games.contains_key(&rematch_id));
        }

        send_disconnect(&source_tx, p0.id, GameAudience::Seat(0)).await;
        send_disconnect(&source_tx, p1.id, GameAudience::Seat(1)).await;
        send_disconnect(&source_tx, p2.id, GameAudience::Seat(2)).await;

        wait_until(|| {
            if let Ok(guard) = runtime.try_read() {
                guard
                    .game_meta
                    .get(&rematch_id)
                    .map(|meta| meta.seats.is_empty())
                    .unwrap_or(false)
            } else {
                false
            }
        })
        .await;

        let guard = runtime.read().await;
        assert!(guard.games.contains_key(&rematch_id));
        assert_eq!(
            guard
                .lobby_cache
                .get(&rematch_id)
                .map(|entry| entry.seat_count),
            Some(0)
        );
        assert_eq!(
            guard
                .lobby_cache
                .get(&rematch_id)
                .map(|entry| entry.seat_user_ids.clone()),
            Some(vec![p0.id, p1.id, p2.id])
        );
        assert_eq!(
            guard
                .game_meta
                .get(&rematch_id)
                .map(|meta| meta.seats.len()),
            Some(0)
        );
    }

    #[tokio::test]
    async fn started_rematch_is_not_removed_by_source_disconnect_cleanup() {
        let runtime = Arc::new(RwLock::new(GlobalRuntimeState::new(200)));
        let (persistence_tx, _persistence_rx) = mpsc::channel(8);
        let p0 = user(20, "p0");
        let p1 = user(21, "p1");
        let p2 = user(22, "p2");
        let source_id = 84;

        let source_tx = spawn_game_actor(
            runtime.clone(),
            persistence_tx.clone(),
            finished_source_game(source_id, &p0, &p1, &p2),
        )
        .await;

        subscribe_as_seat(&source_tx, p0.clone()).await;
        subscribe_as_seat(&source_tx, p1.clone()).await;
        subscribe_as_seat(&source_tx, p2.clone()).await;
        subscribe_as_spectator(&source_tx, p0.clone()).await;

        let rematch_id = create_rematch_for_user(
            runtime.clone(),
            persistence_tx.clone(),
            source_id,
            p0.clone(),
        )
        .await
        .expect("create rematch");
        let rematch_tx = game_sender(&runtime, rematch_id)
            .await
            .expect("rematch sender");

        set_ready_ok(&rematch_tx, p0.clone(), true).await;
        set_ready_ok(&rematch_tx, p1.clone(), true).await;
        set_ready_ok(&rematch_tx, p2.clone(), true).await;

        wait_until(|| {
            if let Ok(guard) = runtime.try_read() {
                guard
                    .game_meta
                    .get(&rematch_id)
                    .map(|meta| meta.status == STATUS_STARTED)
                    .unwrap_or(false)
            } else {
                false
            }
        })
        .await;

        send_disconnect(&source_tx, p0.id, GameAudience::Seat(0)).await;
        send_disconnect(&source_tx, p1.id, GameAudience::Seat(1)).await;
        send_disconnect(&source_tx, p2.id, GameAudience::Seat(2)).await;

        sleep(Duration::from_millis(50)).await;

        let guard = runtime.read().await;
        assert!(guard.games.contains_key(&rematch_id));
        assert_eq!(
            guard.game_meta.get(&rematch_id).map(|meta| meta.status),
            Some(STATUS_STARTED)
        );
        let lobby_snapshot = guard.lobby_tx.borrow();
        let spectatable_game = lobby_snapshot
            .spectatable_games
            .iter()
            .find(|entry| entry.id == rematch_id)
            .expect("started rematch should be spectatable");
        assert_eq!(spectatable_game.rematch_from_game_id, Some(source_id));
    }

    #[tokio::test]
    async fn rematch_creation_reuses_existing_lobby_id() {
        let runtime = Arc::new(RwLock::new(GlobalRuntimeState::new(300)));
        let (persistence_tx, _persistence_rx) = mpsc::channel(8);
        let p0 = user(30, "p0");
        let p1 = user(31, "p1");
        let p2 = user(32, "p2");
        let source_id = 126;

        let _source_tx = spawn_game_actor(
            runtime.clone(),
            persistence_tx.clone(),
            finished_source_game(source_id, &p0, &p1, &p2),
        )
        .await;

        let first_id = create_rematch_for_user(
            runtime.clone(),
            persistence_tx.clone(),
            source_id,
            p0.clone(),
        )
        .await
        .expect("first rematch");
        let second_id = create_rematch_for_user(
            runtime.clone(),
            persistence_tx.clone(),
            source_id,
            p1.clone(),
        )
        .await
        .expect("second rematch");

        assert_eq!(first_id, second_id);
    }

    #[tokio::test]
    async fn rematch_creation_emits_persistence_link_message() {
        let runtime = Arc::new(RwLock::new(GlobalRuntimeState::new(360)));
        let (persistence_tx, mut persistence_rx) = mpsc::channel(8);
        let p0 = user(33, "p0");
        let p1 = user(34, "p1");
        let p2 = user(35, "p2");
        let source_id = 140;

        let _source_tx = spawn_game_actor(
            runtime.clone(),
            persistence_tx.clone(),
            finished_source_game(source_id, &p0, &p1, &p2),
        )
        .await;

        let rematch_id = create_rematch_for_user(
            runtime.clone(),
            persistence_tx.clone(),
            source_id,
            p0.clone(),
        )
        .await
        .expect("create rematch");
        let write = persistence_rx.recv().await.expect("persistence write");
        assert_eq!(
            write,
            PersistenceWriteMessage::LinkRematch {
                source_game_id: source_id,
                next_game_id: rematch_id,
            }
        );
    }

    #[tokio::test]
    async fn rematch_creation_rotates_dealer_clockwise() {
        let runtime = Arc::new(RwLock::new(GlobalRuntimeState::new(400)));
        let (persistence_tx, _persistence_rx) = mpsc::channel(8);
        let p0 = user(40, "p0");
        let p1 = user(41, "p1");
        let p2 = user(42, "p2");
        let source_id = 168;

        let mut source = finished_source_game(source_id, &p0, &p1, &p2);
        source.transcript.dealer = 2;
        source.engine = CutthroatState::new_with_deck(2, source.transcript.deck.clone());

        let _source_tx = spawn_game_actor(runtime.clone(), persistence_tx.clone(), source).await;

        let rematch_id = create_rematch_for_user(
            runtime.clone(),
            persistence_tx.clone(),
            source_id,
            p0.clone(),
        )
        .await
        .expect("create rematch");

        let guard = runtime.read().await;
        assert_eq!(
            guard.game_meta.get(&source_id).map(|meta| meta.dealer),
            Some(2)
        );
        assert_eq!(
            guard.game_meta.get(&rematch_id).map(|meta| meta.dealer),
            Some(0)
        );
    }

    #[tokio::test]
    async fn rematch_leave_and_rejoin_preserves_reserved_seat() {
        let runtime = Arc::new(RwLock::new(GlobalRuntimeState::new(500)));
        let (persistence_tx, _persistence_rx) = mpsc::channel(8);
        let p0 = user(50, "p0");
        let p1 = user(51, "p1");
        let p2 = user(52, "p2");
        let source_id = 210;

        let _source_tx = spawn_game_actor(
            runtime.clone(),
            persistence_tx.clone(),
            finished_source_game(source_id, &p0, &p1, &p2),
        )
        .await;

        let rematch_id = create_rematch_for_user(
            runtime.clone(),
            persistence_tx.clone(),
            source_id,
            p0.clone(),
        )
        .await
        .expect("create rematch");
        let rematch_tx = game_sender(&runtime, rematch_id)
            .await
            .expect("rematch sender");

        leave_game_ok(&rematch_tx, p1.clone()).await;
        let seat = join_game_result(&rematch_tx, p1.clone())
            .await
            .expect("p1 rejoin should succeed");
        assert_eq!(seat, 1);
    }

    #[tokio::test]
    async fn rematch_disconnect_and_rejoin_preserves_reserved_seat() {
        let runtime = Arc::new(RwLock::new(GlobalRuntimeState::new(550)));
        let (persistence_tx, _persistence_rx) = mpsc::channel(8);
        let p0 = user(55, "p0");
        let p1 = user(56, "p1");
        let p2 = user(57, "p2");
        let source_id = 220;

        let source_tx = spawn_game_actor(
            runtime.clone(),
            persistence_tx.clone(),
            finished_source_game(source_id, &p0, &p1, &p2),
        )
        .await;

        subscribe_as_seat(&source_tx, p0.clone()).await;
        subscribe_as_seat(&source_tx, p1.clone()).await;
        subscribe_as_seat(&source_tx, p2.clone()).await;

        let rematch_id = create_rematch_for_user(
            runtime.clone(),
            persistence_tx.clone(),
            source_id,
            p0.clone(),
        )
        .await
        .expect("create rematch");
        let rematch_tx = game_sender(&runtime, rematch_id)
            .await
            .expect("rematch sender");

        send_disconnect(&source_tx, p1.id, GameAudience::Seat(1)).await;
        wait_until(|| {
            if let Ok(guard) = runtime.try_read() {
                guard
                    .game_meta
                    .get(&rematch_id)
                    .map(|meta| !meta.seats.iter().any(|seat| seat.user_id == p1.id))
                    .unwrap_or(false)
            } else {
                false
            }
        })
        .await;

        {
            let guard = runtime.read().await;
            assert_eq!(
                guard
                    .lobby_cache
                    .get(&rematch_id)
                    .map(|entry| entry.seat_count),
                Some(2)
            );
            assert_eq!(
                guard
                    .lobby_cache
                    .get(&rematch_id)
                    .map(|entry| entry.seat_user_ids.clone()),
                Some(vec![p0.id, p1.id, p2.id])
            );
        }

        let seat = join_game_result(&rematch_tx, p1.clone())
            .await
            .expect("p1 rejoin should succeed");
        assert_eq!(seat, 1);
    }

    #[tokio::test]
    async fn rematch_rejects_non_series_user_for_source_disconnect_vacated_seat() {
        let runtime = Arc::new(RwLock::new(GlobalRuntimeState::new(600)));
        let (persistence_tx, _persistence_rx) = mpsc::channel(8);
        let p0 = user(60, "p0");
        let p1 = user(61, "p1");
        let p2 = user(62, "p2");
        let outsider = user(99, "outsider");
        let source_id = 252;

        let source_tx = spawn_game_actor(
            runtime.clone(),
            persistence_tx.clone(),
            finished_source_game(source_id, &p0, &p1, &p2),
        )
        .await;
        subscribe_as_seat(&source_tx, p0.clone()).await;
        subscribe_as_seat(&source_tx, p1.clone()).await;
        subscribe_as_seat(&source_tx, p2.clone()).await;

        let rematch_id = create_rematch_for_user(
            runtime.clone(),
            persistence_tx.clone(),
            source_id,
            p0.clone(),
        )
        .await
        .expect("create rematch");
        let rematch_tx = game_sender(&runtime, rematch_id)
            .await
            .expect("rematch sender");

        send_disconnect(&source_tx, p1.id, GameAudience::Seat(1)).await;
        wait_until(|| {
            if let Ok(guard) = runtime.try_read() {
                guard
                    .game_meta
                    .get(&rematch_id)
                    .map(|meta| !meta.seats.iter().any(|seat| seat.user_id == p1.id))
                    .unwrap_or(false)
            } else {
                false
            }
        })
        .await;

        let join_result = join_game_result(&rematch_tx, outsider).await;
        assert!(matches!(join_result, Err(RuntimeError::Forbidden)));
    }

    #[tokio::test]
    async fn rematch_requires_rejoin_before_ready_and_start_after_source_disconnect() {
        let runtime = Arc::new(RwLock::new(GlobalRuntimeState::new(650)));
        let (persistence_tx, _persistence_rx) = mpsc::channel(8);
        let p0 = user(70, "p0");
        let p1 = user(71, "p1");
        let p2 = user(72, "p2");
        let source_id = 262;

        let source_tx = spawn_game_actor(
            runtime.clone(),
            persistence_tx.clone(),
            finished_source_game(source_id, &p0, &p1, &p2),
        )
        .await;

        subscribe_as_seat(&source_tx, p0.clone()).await;
        subscribe_as_seat(&source_tx, p1.clone()).await;
        subscribe_as_seat(&source_tx, p2.clone()).await;

        let rematch_id = create_rematch_for_user(
            runtime.clone(),
            persistence_tx.clone(),
            source_id,
            p0.clone(),
        )
        .await
        .expect("create rematch");
        let rematch_tx = game_sender(&runtime, rematch_id)
            .await
            .expect("rematch sender");

        send_disconnect(&source_tx, p2.id, GameAudience::Seat(2)).await;
        wait_until(|| {
            if let Ok(guard) = runtime.try_read() {
                guard
                    .game_meta
                    .get(&rematch_id)
                    .map(|meta| !meta.seats.iter().any(|seat| seat.user_id == p2.id))
                    .unwrap_or(false)
            } else {
                false
            }
        })
        .await;

        set_ready_ok(&rematch_tx, p0.clone(), true).await;
        set_ready_ok(&rematch_tx, p1.clone(), true).await;
        let ready_result = {
            let (resp_tx, resp_rx) = tokio::sync::oneshot::channel();
            rematch_tx
                .send(GameCommand::SetReady {
                    user: p2.clone(),
                    ready: true,
                    respond: resp_tx,
                })
                .await
                .expect("set ready send");
            resp_rx.await.expect("set ready recv")
        };
        assert!(matches!(ready_result, Err(RuntimeError::Forbidden)));

        let start_result = {
            let (resp_tx, resp_rx) = tokio::sync::oneshot::channel();
            rematch_tx
                .send(GameCommand::StartGame {
                    user: p0.clone(),
                    respond: resp_tx,
                })
                .await
                .expect("start game send");
            resp_rx.await.expect("start game recv")
        };
        assert!(matches!(start_result, Err(RuntimeError::Conflict)));

        let seat = join_game_result(&rematch_tx, p2.clone())
            .await
            .expect("p2 rejoin should succeed");
        assert_eq!(seat, 2);
        set_ready_ok(&rematch_tx, p2.clone(), true).await;
        wait_until(|| {
            if let Ok(guard) = runtime.try_read() {
                guard
                    .game_meta
                    .get(&rematch_id)
                    .map(|meta| meta.status == STATUS_STARTED)
                    .unwrap_or(false)
            } else {
                false
            }
        })
        .await;
    }
}
