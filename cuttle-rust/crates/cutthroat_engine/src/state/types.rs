use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::action::Action;
use crate::card::Card;

pub type Seat = u8;

pub const PLAYER_COUNT: u8 = 3;
pub const HAND_LIMIT: usize = 7;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Winner {
    Seat(Seat),
    Draw,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Phase {
    Main,
    Countering(CounterState),
    ResolvingThree {
        seat: Seat,
        base_player: Seat,
    },
    ResolvingFour {
        seat: Seat,
        base_player: Seat,
        remaining: u8,
    },
    ResolvingFive {
        seat: Seat,
        base_player: Seat,
        discarded: bool,
    },
    ResolvingSeven {
        seat: Seat,
        base_player: Seat,
        revealed: Vec<Card>,
    },
    GameOver,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CounterState {
    pub base_player: Seat,
    pub oneoff: Action,
    pub twos: Vec<(Seat, Card)>,
    pub next_seat: Seat,
    pub rotation_anchor: Seat,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FrozenCard {
    pub card: Card,
    pub remaining_turns: u8,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JackOnStack {
    pub card: Card,
    pub owner: Seat,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JokerOnStack {
    pub card: Card,
    pub owner: Seat,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PointStack {
    pub base: Card,
    pub base_owner: Seat,
    pub jacks: Vec<JackOnStack>,
}

impl PointStack {
    pub fn controller(&self) -> Seat {
        self.jacks
            .last()
            .map(|j| j.owner)
            .unwrap_or(self.base_owner)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RoyalStack {
    pub base: Card,
    pub base_owner: Seat,
    pub jokers: Vec<JokerOnStack>,
}

impl RoyalStack {
    pub fn controller(&self) -> Seat {
        self.jokers
            .last()
            .map(|j| j.owner)
            .unwrap_or(self.base_owner)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlayerState {
    pub hand: Vec<Card>,
    pub points: Vec<PointStack>,
    pub royals: Vec<RoyalStack>,
    pub frozen: Vec<FrozenCard>,
}

impl PlayerState {
    pub fn new() -> Self {
        Self {
            hand: Vec::new(),
            points: Vec::new(),
            royals: Vec::new(),
            frozen: Vec::new(),
        }
    }
}

impl Default for PlayerState {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CutthroatState {
    pub dealer: Seat,
    pub turn: Seat,
    pub phase: Phase,
    pub deck: Vec<Card>,
    pub scrap: Vec<Card>,
    pub players: Vec<PlayerState>,
    pub pass_streak_start: Option<Seat>,
    pub pass_streak_len: u8,
    pub winner: Option<Winner>,
}

#[derive(Clone, Debug, Error)]
pub enum RuleError {
    #[error("not your turn")]
    NotYourTurn,
    #[error("illegal action")]
    IllegalAction,
    #[error("invalid action")]
    InvalidAction,
    #[error("game over")]
    GameOver,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Event {
    GameOver(Winner),
}
