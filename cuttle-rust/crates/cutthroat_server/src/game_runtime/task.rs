use crate::game_runtime::{Command, GameRuntime};
use crate::persistence::CompletedGameRecord;
use tokio::sync::mpsc;
use tracing::error;

pub(crate) async fn runtime_task(
    mut rx: mpsc::Receiver<Command>,
    persistence_tx: mpsc::Sender<CompletedGameRecord>,
    initial_next_game_id: i64,
) {
    let mut store = GameRuntime::new(initial_next_game_id);

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
            Command::StartGame {
                game_id,
                user,
                respond,
            } => {
                let result = store.start_game(game_id, user);
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
            Command::GetSpectateReplayState {
                game_id,
                user,
                game_state_index,
                respond,
            } => {
                let result =
                    store.build_spectator_replay_state_for_user(game_id, &user, game_state_index);
                let _ = respond.send(result);
            }
            Command::SubscribeGameStream {
                game_id,
                user,
                spectate_intent,
                respond,
            } => {
                let result = store.subscribe_game_stream(game_id, user, spectate_intent);
                let _ = respond.send(result);
            }
            Command::SubscribeLobbyStream { respond } => {
                let _ = respond.send(store.subscribe_lobby_stream());
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
            #[cfg(feature = "e2e-seed")]
            Command::SeedGameFromTokenlog { seed, respond } => {
                let result = store.seed_game_from_tokenlog(seed);
                let _ = respond.send(result);
            }
        }
    }
}
