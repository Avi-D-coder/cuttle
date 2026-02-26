use super::types::{Phase, Seat};
use crate::action::Action;
use crate::tokens::Token;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PublicCard {
    Hidden,
    Known(Token),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PointStackView {
    pub base: Token,
    pub controller: Seat,
    pub jacks: Vec<Token>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoyalStackView {
    pub base: Token,
    pub controller: Seat,
    pub jokers: Vec<Token>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerView {
    pub seat: Seat,
    pub hand: Vec<PublicCard>,
    pub points: Vec<PointStackView>,
    pub royals: Vec<RoyalStackView>,
    pub frozen: Vec<Token>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SeatView {
    pub seat: Seat,
    pub turn: Seat,
    pub phase: PhaseView,
    pub deck_count: usize,
    pub scrap: Vec<Token>,
    pub players: Vec<PlayerView>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CounterTwoView {
    pub seat: Seat,
    pub card: Token,
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
        revealed_cards: Vec<Token>,
    },
    GameOver,
}

impl Phase {
    pub(crate) fn view(&self, _viewer: Seat) -> PhaseView {
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
                        card: card.to_token_enum(),
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
                revealed_cards: revealed.iter().map(|card| card.to_token_enum()).collect(),
            },
            Phase::GameOver => PhaseView::GameOver,
        }
    }
}
