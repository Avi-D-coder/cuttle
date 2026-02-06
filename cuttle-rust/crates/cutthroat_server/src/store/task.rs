use crate::persistence::CompletedGameRecord;
use crate::state::{GameUpdate, LobbyListUpdate, ScrapStraightenUpdate};
use crate::store::{Command, Store};
use tokio::sync::{broadcast, mpsc};
use tracing::error;

pub(crate) async fn store_task(
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
