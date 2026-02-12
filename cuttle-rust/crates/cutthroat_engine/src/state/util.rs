use super::types::{PLAYER_COUNT, Seat};
use crate::card::{Card, Rank};

pub(crate) fn is_point_card(card: Card) -> bool {
    matches!(
        card,
        Card::Standard {
            rank: Rank::Ace
                | Rank::Two
                | Rank::Three
                | Rank::Four
                | Rank::Five
                | Rank::Six
                | Rank::Seven
                | Rank::Eight
                | Rank::Nine
                | Rank::Ten,
            ..
        }
    )
}

pub(crate) fn next_seat(seat: Seat) -> Seat {
    (seat + 1) % PLAYER_COUNT
}
