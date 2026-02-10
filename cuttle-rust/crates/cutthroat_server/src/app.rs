#[cfg(feature = "e2e-seed")]
use crate::api::handlers::seed_game_from_tokenlog;
#[cfg(feature = "e2e-seed")]
use crate::api::handlers::seed_game_from_transcript;
use crate::api::handlers::{
    create_game, get_health, get_history, get_spectate_state, get_state, join_game, leave_game,
    post_action, rematch_game, set_ready, start_game,
};
use crate::state::AppState;
use crate::ws::{ws_handler, ws_lobbies_handler, ws_spectate_handler};
use axum::{
    Router,
    routing::{get, post},
};

pub(crate) fn build_router(state: AppState) -> Router {
    let router = Router::new()
        .route("/cutthroat/api/v1/health", get(get_health))
        .route("/cutthroat/api/v1/games", post(create_game))
        .route("/cutthroat/api/v1/games/{id}/join", post(join_game))
        .route("/cutthroat/api/v1/games/{id}/leave", post(leave_game))
        .route("/cutthroat/api/v1/games/{id}/rematch", post(rematch_game))
        .route("/cutthroat/api/v1/games/{id}/ready", post(set_ready))
        .route("/cutthroat/api/v1/games/{id}/start", post(start_game))
        .route("/cutthroat/api/v1/games/{id}/state", get(get_state))
        .route("/cutthroat/api/v1/history", get(get_history))
        .route(
            "/cutthroat/api/v1/games/{id}/spectate/state",
            get(get_spectate_state),
        )
        .route("/cutthroat/api/v1/games/{id}/action", post(post_action))
        .route("/cutthroat/ws/games/{id}", get(ws_handler))
        .route(
            "/cutthroat/ws/games/{id}/spectate",
            get(ws_spectate_handler),
        )
        .route("/cutthroat/ws/lobbies", get(ws_lobbies_handler));

    #[cfg(feature = "e2e-seed")]
    let router = router
        .route(
            "/cutthroat/api/test/games/seed-tokenlog",
            post(seed_game_from_tokenlog),
        )
        .route(
            "/cutthroat/api/test/games/seed-transcript",
            post(seed_game_from_transcript),
        );

    router.with_state(state)
}
