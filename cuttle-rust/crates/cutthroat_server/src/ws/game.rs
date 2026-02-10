use crate::api::handlers::{
    apply_action_with_sender, set_socket_disconnected, subscribe_game_stream,
    toggle_scrap_straighten_with_sender,
};
use crate::auth::authorize;
use crate::state::AppState;
use crate::ws::messages::{WsClientMessage, WsServerMessage};
use axum::extract::ws::{Message, WebSocket};
use axum::{
    extract::{Path, State, WebSocketUpgrade},
    http::HeaderMap,
    response::IntoResponse,
};
use std::sync::Arc;
use tokio::time::{Duration, timeout};

const WS_SEND_TIMEOUT_SECS: u64 = 3;

pub(crate) async fn ws_handler(
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

pub(crate) async fn ws_spectate_handler(
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

async fn send_server_message(socket: &mut WebSocket, message: WsServerMessage) -> bool {
    let payload = match serde_json::to_string(&message) {
        Ok(payload) => payload,
        Err(_) => return false,
    };

    matches!(
        timeout(
            Duration::from_secs(WS_SEND_TIMEOUT_SECS),
            socket.send(Message::Text(payload.into())),
        )
        .await,
        Ok(Ok(()))
    )
}

async fn send_state(
    socket: &mut WebSocket,
    state: Arc<crate::api::handlers::GameStateResponse>,
) -> bool {
    send_server_message(
        socket,
        WsServerMessage::State {
            state: Box::new((*state).clone()),
        },
    )
    .await
}

async fn send_error(socket: &mut WebSocket, code: u16, message: String) {
    let _ = send_server_message(socket, WsServerMessage::Error { code, message }).await;
}

async fn handle_ws(
    mut socket: WebSocket,
    state: AppState,
    game_id: i64,
    user: crate::auth::AuthUser,
    spectate_intent: bool,
) {
    let (sender, subscription) =
        match subscribe_game_stream(&state, game_id, user.clone(), spectate_intent).await {
            Ok(subscription) => subscription,
            Err((code, message)) => {
                send_error(&mut socket, code, message).await;
                return;
            }
        };

    let mut state_rx = subscription.rx;

    let initial_state = {
        let borrowed = state_rx.borrow();
        borrowed.clone()
    };
    if !send_state(&mut socket, initial_state).await {
        set_socket_disconnected(&sender, user.id, subscription.audience).await;
        return;
    }

    loop {
        tokio::select! {
            changed = state_rx.changed() => {
                if changed.is_err() {
                    break;
                }
                let latest = {
                    let borrowed = state_rx.borrow_and_update();
                    borrowed.clone()
                };
                if !send_state(&mut socket, latest).await {
                    break;
                }
            }
            msg = socket.recv() => {
                let Some(msg) = msg else { break; };
                let Ok(msg) = msg else { break; };
                match msg {
                    Message::Text(text) => {
                        match serde_json::from_str::<WsClientMessage>(&text) {
                            Ok(WsClientMessage::Action {
                                expected_version,
                                action_tokens,
                            }) => {
                                let result = apply_action_with_sender(
                                    &sender,
                                    user.clone(),
                                    expected_version,
                                    action_tokens,
                                ).await;
                                if let Err((code, message)) = result {
                                    send_error(&mut socket, code, message).await;
                                }
                            }
                            Ok(WsClientMessage::ScrapStraighten) => {
                                let result = toggle_scrap_straighten_with_sender(&sender, user.clone()).await;
                                if let Err((code, message)) = result {
                                    send_error(&mut socket, code, message).await;
                                }
                            }
                            Err(err) => {
                                send_error(&mut socket, 400, format!("invalid message: {}", err)).await;
                            }
                        }
                    }
                    Message::Close(_) => break,
                    _ => {}
                }
            }
        }
    }

    set_socket_disconnected(&sender, user.id, subscription.audience).await;
}
