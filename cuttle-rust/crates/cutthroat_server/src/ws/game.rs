use crate::api::handlers::{
    apply_action_internal, build_state_response_for_user, set_spectator_connected,
    set_spectator_disconnected, toggle_scrap_straighten_internal, validate_viewer,
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

async fn handle_ws(
    mut socket: WebSocket,
    state: AppState,
    game_id: i64,
    user: crate::auth::AuthUser,
    spectate_intent: bool,
) {
    let mut updates = state.updates.subscribe();
    let mut scrap_straighten_updates = state.scrap_straighten_updates.subscribe();
    let mut spectator_registered = false;
    if let Err((code, message)) =
        validate_viewer(&state, game_id, user.clone(), spectate_intent).await
    {
        let _ = socket
            .send(Message::Text(
                serde_json::to_string(&WsServerMessage::Error { code, message })
                    .unwrap()
                    .into(),
            ))
            .await;
        return;
    }

    match build_state_response_for_user(&state, game_id, user.clone(), spectate_intent).await {
        Ok(resp) => {
            if resp.is_spectator {
                if set_spectator_connected(&state, game_id, user.clone())
                    .await
                    .is_err()
                {
                    return;
                }
                spectator_registered = true;
            }
            if socket
                .send(Message::Text(
                    serde_json::to_string(&WsServerMessage::State(Box::new(resp)))
                        .unwrap()
                        .into(),
                ))
                .await
                .is_err()
            {
                if spectator_registered {
                    set_spectator_disconnected(&state, game_id, user.id).await;
                }
                return;
            }
        }
        Err(_) => return,
    }

    loop {
        tokio::select! {
            update = updates.recv() => {
                let Ok(update) = update else { break; };
                if update.game_id != game_id {
                    continue;
                }
                if let Ok(resp) = build_state_response_for_user(&state, game_id, user.clone(), spectate_intent).await
                    && socket
                        .send(Message::Text(
                            serde_json::to_string(&WsServerMessage::State(Box::new(resp)))
                                .unwrap()
                                .into(),
                        ))
                        .await
                        .is_err()
                {
                    break;
                }
            }
            update = scrap_straighten_updates.recv() => {
                let Ok(update) = update else { break; };
                if update.game_id != game_id {
                    continue;
                }
                if socket
                    .send(Message::Text(
                        serde_json::to_string(&WsServerMessage::ScrapStraighten {
                            game_id: update.game_id,
                            straightened: update.straightened,
                            actor_seat: update.actor_seat,
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
                match msg {
                    Message::Text(text) => {
                        match serde_json::from_str::<WsClientMessage>(&text) {
                            Ok(WsClientMessage::Action { expected_version, action }) => {
                                let result = apply_action_internal(&state, game_id, user.clone(), expected_version, action).await;
                                if let Err((code, message)) = result {
                                    let _ = socket
                                        .send(Message::Text(
                                            serde_json::to_string(&WsServerMessage::Error { code, message })
                                                .unwrap()
                                                .into(),
                                        ))
                                        .await;
                                }
                            }
                            Ok(WsClientMessage::ScrapStraighten) => {
                                let result = toggle_scrap_straighten_internal(&state, game_id, user.clone()).await;
                                if let Err((code, message)) = result {
                                    let _ = socket
                                        .send(Message::Text(
                                            serde_json::to_string(&WsServerMessage::Error { code, message })
                                                .unwrap()
                                                .into(),
                                        ))
                                        .await;
                                }
                            }
                            Err(err) => {
                                let _ = socket
                                    .send(Message::Text(
                                        serde_json::to_string(&WsServerMessage::Error {
                                            code: 400,
                                            message: format!("invalid message: {}", err),
                                        })
                                        .unwrap()
                                        .into(),
                                    ))
                                    .await;
                            }
                        }
                    }
                    Message::Close(_) => break,
                    _ => {}
                }
            }
        }
    }
    if spectator_registered {
        set_spectator_disconnected(&state, game_id, user.id).await;
    }
}
