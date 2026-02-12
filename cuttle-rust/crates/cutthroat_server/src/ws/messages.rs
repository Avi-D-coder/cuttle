use crate::api::handlers::{GameStateResponse, LobbySummary, SpectatableGameSummary};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
#[serde(tag = "type")]
pub(crate) enum WsClientMessage {
    #[serde(rename = "action")]
    Action {
        expected_version: i64,
        action_tokens: String,
    },
    #[serde(rename = "scrap_straighten")]
    ScrapStraighten,
}

#[derive(Serialize)]
#[serde(tag = "type")]
pub(crate) enum WsServerMessage {
    #[serde(rename = "state")]
    State { state: Box<GameStateResponse> },
    #[serde(rename = "lobbies")]
    Lobbies {
        version: u64,
        lobbies: Vec<LobbySummary>,
        spectatable_games: Vec<SpectatableGameSummary>,
    },
    #[serde(rename = "error")]
    Error { code: u16, message: String },
}
