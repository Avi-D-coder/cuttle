use crate::api::handlers::{LobbySummary, SpectatableGameSummary, subscribe_lobby_stream};
use crate::auth::authorize;
use crate::game_runtime::LobbySnapshotInternal;
use crate::state::AppState;
use crate::ws::messages::WsServerMessage;
use axum::extract::ws::Message;
use axum::{
    extract::{State, WebSocketUpgrade},
    http::HeaderMap,
    response::IntoResponse,
};
use std::sync::Arc;
use tokio::time::{Duration, timeout};

const WS_SEND_TIMEOUT_SECS: u64 = 3;

pub(crate) async fn ws_lobbies_handler(
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

fn filter_for_user(
    snapshot: &LobbySnapshotInternal,
    user_id: i64,
) -> (u64, Vec<LobbySummary>, Vec<SpectatableGameSummary>) {
    let lobbies = snapshot
        .lobbies
        .iter()
        .filter(|entry| !entry.is_rematch_lobby || entry.seat_user_ids.contains(&user_id))
        .map(|entry| entry.summary.clone())
        .collect();

    (
        snapshot.version,
        lobbies,
        snapshot.spectatable_games.clone(),
    )
}

async fn send_lobbies_message(
    socket: &mut axum::extract::ws::WebSocket,
    payload: Arc<LobbySnapshotInternal>,
    user_id: i64,
) -> bool {
    let (version, lobbies, spectatable_games) = filter_for_user(&payload, user_id);
    let message = WsServerMessage::Lobbies {
        version,
        lobbies,
        spectatable_games,
    };
    let encoded = match serde_json::to_string(&message) {
        Ok(encoded) => encoded,
        Err(_) => return false,
    };

    matches!(
        timeout(
            Duration::from_secs(WS_SEND_TIMEOUT_SECS),
            socket.send(Message::Text(encoded.into())),
        )
        .await,
        Ok(Ok(()))
    )
}

async fn handle_lobbies_ws(
    mut socket: axum::extract::ws::WebSocket,
    state: AppState,
    user: crate::auth::AuthUser,
) {
    let mut updates = match subscribe_lobby_stream(&state).await {
        Ok(rx) => rx,
        Err(_) => return,
    };

    let initial_snapshot = {
        let borrowed = updates.borrow();
        borrowed.clone()
    };
    if !send_lobbies_message(&mut socket, initial_snapshot, user.id).await {
        return;
    }

    loop {
        tokio::select! {
            changed = updates.changed() => {
                if changed.is_err() {
                    break;
                }
                let payload = {
                    let borrowed = updates.borrow_and_update();
                    borrowed.clone()
                };
                if !send_lobbies_message(&mut socket, payload, user.id).await {
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
