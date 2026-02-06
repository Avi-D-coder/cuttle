use crate::api::handlers::lobby_list_for_user;
use crate::auth::authorize;
use crate::state::AppState;
use crate::ws::messages::WsServerMessage;
use axum::extract::ws::Message;
use axum::{
    extract::{State, WebSocketUpgrade},
    http::HeaderMap,
    response::IntoResponse,
};

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

async fn handle_lobbies_ws(
    mut socket: axum::extract::ws::WebSocket,
    state: AppState,
    user: crate::auth::AuthUser,
) {
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
