use super::types::{Phase, Seat};
use crate::action::Action;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PublicCard {
    Hidden,
    Known(String),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PointStackView {
    pub base: String,
    pub controller: Seat,
    pub jacks: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoyalStackView {
    pub base: String,
    pub controller: Seat,
    pub jokers: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerView {
    pub seat: Seat,
    pub hand: Vec<PublicCard>,
    pub points: Vec<PointStackView>,
    pub royals: Vec<RoyalStackView>,
    pub frozen: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublicView {
    pub seat: Seat,
    pub turn: Seat,
    pub phase: PhaseView,
    pub deck_count: usize,
    pub scrap: Vec<String>,
    pub players: Vec<PlayerView>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_event: Option<LastEventView>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LastEventView {
    pub actor: Seat,
    pub action_kind: String,
    pub change: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_token: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_zone: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_token: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_seat: Option<Seat>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub oneoff_rank: Option<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CounterTwoView {
    pub seat: Seat,
    pub card: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum PhaseView {
    Main,
    Countering {
        next_seat: Seat,
        base_player: Seat,
        oneoff: Action,
        twos: Vec<CounterTwoView>,
    },
    ResolvingThree {
        seat: Seat,
    },
    ResolvingFour {
        seat: Seat,
        remaining: u8,
    },
    ResolvingFive {
        seat: Seat,
    },
    ResolvingSeven {
        seat: Seat,
        revealed: usize,
        revealed_cards: Vec<String>,
    },
    GameOver,
}

impl Phase {
    pub(crate) fn view(&self, viewer: Seat) -> PhaseView {
        match self {
            Phase::Main => PhaseView::Main,
            Phase::Countering(counter) => PhaseView::Countering {
                next_seat: counter.next_seat,
                base_player: counter.base_player,
                oneoff: counter.oneoff.clone(),
                twos: counter
                    .twos
                    .iter()
                    .map(|(seat, card)| CounterTwoView {
                        seat: *seat,
                        card: card.to_token(),
                    })
                    .collect(),
            },
            Phase::ResolvingThree { seat, .. } => PhaseView::ResolvingThree { seat: *seat },
            Phase::ResolvingFour {
                seat, remaining, ..
            } => PhaseView::ResolvingFour {
                seat: *seat,
                remaining: *remaining,
            },
            Phase::ResolvingFive { seat, .. } => PhaseView::ResolvingFive { seat: *seat },
            Phase::ResolvingSeven { seat, revealed, .. } => PhaseView::ResolvingSeven {
                seat: *seat,
                revealed: revealed.len(),
                revealed_cards: if *seat == viewer {
                    revealed.iter().map(|card| card.to_token()).collect()
                } else {
                    Vec::new()
                },
            },
            Phase::GameOver => PhaseView::GameOver,
        }
    }
}
