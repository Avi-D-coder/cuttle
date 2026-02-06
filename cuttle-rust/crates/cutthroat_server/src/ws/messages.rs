use crate::api::handlers::{GameStateResponse, LobbySummary, SpectatableGameSummary};
use cutthroat_engine::{Action, Seat};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
#[serde(tag = "type")]
pub(crate) enum WsClientMessage {
    #[serde(rename = "action")]
    Action {
        expected_version: i64,
        action: Action,
    },
    #[serde(rename = "scrap_straighten")]
    ScrapStraighten,
}

#[derive(Serialize)]
#[serde(tag = "type")]
pub(crate) enum WsServerMessage {
    #[serde(rename = "state")]
    State(Box<GameStateResponse>),
    #[serde(rename = "lobbies")]
    Lobbies {
        lobbies: Vec<LobbySummary>,
        spectatable_games: Vec<SpectatableGameSummary>,
    },
    #[serde(rename = "scrap_straighten")]
    ScrapStraighten {
        game_id: i64,
        straightened: bool,
        actor_seat: Seat,
    },
    #[serde(rename = "error")]
    Error { code: u16, message: String },
}
