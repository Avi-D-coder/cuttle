use serde::{Deserialize, Serialize};

use crate::card::Card;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum OneOffTarget {
    None,
    Player { seat: u8 },
    Point { base: Card },
    Royal { card: Card },
    Jack { card: Card },
    Joker { card: Card },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum SevenPlay {
    Points,
    Scuttle { target: Card },
    Royal,
    Jack { target: Card },
    Joker { target: Card },
    OneOff { target: OneOffTarget },
    Discard,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum Action {
    Draw,
    Pass,
    PlayPoints {
        card: Card,
    },
    Scuttle {
        card: Card,
        target_point_base: Card,
    },
    PlayRoyal {
        card: Card,
    },
    PlayJack {
        jack: Card,
        target_point_base: Card,
    },
    PlayJoker {
        joker: Card,
        target_royal_card: Card,
    },
    PlayOneOff {
        card: Card,
        target: OneOffTarget,
    },
    CounterTwo {
        two_card: Card,
    },
    CounterPass,
    ResolveThreePick {
        card_from_scrap: Card,
    },
    ResolveFourDiscard {
        card: Card,
    },
    ResolveFiveDiscard {
        card: Card,
    },
    ResolveSevenChoose {
        source_index: u8,
        play: SevenPlay,
    },
}
