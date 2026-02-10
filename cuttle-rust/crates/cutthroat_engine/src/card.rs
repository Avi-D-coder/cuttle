use crate::tokens::Token;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Suit {
    Clubs,
    Diamonds,
    Hearts,
    Spades,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Rank {
    Ace,
    Two,
    Three,
    Four,
    Five,
    Six,
    Seven,
    Eight,
    Nine,
    Ten,
    Jack,
    Queen,
    King,
}

impl Rank {
    pub fn from_char(c: char) -> Option<Self> {
        match c {
            'A' => Some(Rank::Ace),
            '2' => Some(Rank::Two),
            '3' => Some(Rank::Three),
            '4' => Some(Rank::Four),
            '5' => Some(Rank::Five),
            '6' => Some(Rank::Six),
            '7' => Some(Rank::Seven),
            '8' => Some(Rank::Eight),
            '9' => Some(Rank::Nine),
            'T' => Some(Rank::Ten),
            'J' => Some(Rank::Jack),
            'Q' => Some(Rank::Queen),
            'K' => Some(Rank::King),
            _ => None,
        }
    }

    pub fn to_char(self) -> char {
        match self {
            Rank::Ace => 'A',
            Rank::Two => '2',
            Rank::Three => '3',
            Rank::Four => '4',
            Rank::Five => '5',
            Rank::Six => '6',
            Rank::Seven => '7',
            Rank::Eight => '8',
            Rank::Nine => '9',
            Rank::Ten => 'T',
            Rank::Jack => 'J',
            Rank::Queen => 'Q',
            Rank::King => 'K',
        }
    }

    pub fn value(self) -> u8 {
        match self {
            Rank::Ace => 1,
            Rank::Two => 2,
            Rank::Three => 3,
            Rank::Four => 4,
            Rank::Five => 5,
            Rank::Six => 6,
            Rank::Seven => 7,
            Rank::Eight => 8,
            Rank::Nine => 9,
            Rank::Ten => 10,
            Rank::Jack => 11,
            Rank::Queen => 12,
            Rank::King => 13,
        }
    }
}

impl Suit {
    pub fn from_char(c: char) -> Option<Self> {
        match c {
            'C' => Some(Suit::Clubs),
            'D' => Some(Suit::Diamonds),
            'H' => Some(Suit::Hearts),
            'S' => Some(Suit::Spades),
            _ => None,
        }
    }

    pub fn to_char(self) -> char {
        match self {
            Suit::Clubs => 'C',
            Suit::Diamonds => 'D',
            Suit::Hearts => 'H',
            Suit::Spades => 'S',
        }
    }

    pub fn order(self) -> u8 {
        match self {
            Suit::Clubs => 0,
            Suit::Diamonds => 1,
            Suit::Hearts => 2,
            Suit::Spades => 3,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Card {
    Standard { rank: Rank, suit: Suit },
    Joker(u8),
}

impl Serialize for Card {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_token())
    }
}

impl<'de> Deserialize<'de> for Card {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Card::from_token(&s).ok_or_else(|| serde::de::Error::custom("invalid card token"))
    }
}

impl Card {
    pub fn from_token(token: &str) -> Option<Self> {
        Token::from_str(token).and_then(Token::card)
    }

    pub fn to_token(self) -> String {
        self.to_token_enum().as_str().to_string()
    }

    pub fn to_token_enum(self) -> Token {
        Token::from_card(self)
    }

    pub fn from_token_enum(token: Token) -> Option<Self> {
        token.card()
    }

    pub fn is_number(self) -> bool {
        matches!(
            self,
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

    pub fn is_oneoff(self) -> bool {
        matches!(
            self,
            Card::Standard {
                rank: Rank::Ace
                    | Rank::Two
                    | Rank::Three
                    | Rank::Four
                    | Rank::Five
                    | Rank::Six
                    | Rank::Seven
                    | Rank::Nine,
                ..
            }
        )
    }

    pub fn is_royal(self) -> bool {
        match self {
            Card::Joker(_) => true,
            Card::Standard { rank, .. } => {
                matches!(rank, Rank::Eight | Rank::Jack | Rank::Queen | Rank::King)
            }
        }
    }

    pub fn rank_value(self) -> Option<u8> {
        match self {
            Card::Standard { rank, .. } => Some(rank.value()),
            Card::Joker(_) => None,
        }
    }

    pub fn suit_order(self) -> Option<u8> {
        match self {
            Card::Standard { suit, .. } => Some(suit.order()),
            Card::Joker(_) => None,
        }
    }

    pub fn point_value(self) -> u8 {
        match self {
            Card::Standard { rank, .. } => match rank {
                Rank::Ace => 1,
                Rank::Two => 2,
                Rank::Three => 3,
                Rank::Four => 4,
                Rank::Five => 5,
                Rank::Six => 6,
                Rank::Seven => 7,
                Rank::Eight => 8,
                Rank::Nine => 9,
                Rank::Ten => 10,
                _ => 0,
            },
            Card::Joker(_) => 0,
        }
    }

    pub fn scuttle_beats(self, defender: Card) -> bool {
        let Some(atk_rank) = self.rank_value() else {
            return false;
        };
        let Some(def_rank) = defender.rank_value() else {
            return false;
        };
        if atk_rank > def_rank {
            return true;
        }
        if atk_rank < def_rank {
            return false;
        }
        let Some(atk_suit) = self.suit_order() else {
            return false;
        };
        let Some(def_suit) = defender.suit_order() else {
            return false;
        };
        atk_suit > def_suit
    }
}

pub fn full_deck_with_jokers() -> Vec<Card> {
    let mut deck = Vec::with_capacity(54);
    for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        for rank in [
            Rank::Ace,
            Rank::Two,
            Rank::Three,
            Rank::Four,
            Rank::Five,
            Rank::Six,
            Rank::Seven,
            Rank::Eight,
            Rank::Nine,
            Rank::Ten,
            Rank::Jack,
            Rank::Queen,
            Rank::King,
        ] {
            deck.push(Card::Standard { rank, suit });
        }
    }
    deck.push(Card::Joker(0));
    deck.push(Card::Joker(1));
    deck
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn card_token_roundtrip() {
        let card = Card::Standard {
            rank: Rank::Seven,
            suit: Suit::Hearts,
        };
        let tok = card.to_token();
        assert_eq!(Card::from_token(&tok), Some(card));
        assert_eq!(Card::from_token("J0"), Some(Card::Joker(0)));
        assert_eq!(Card::from_token("J1"), Some(Card::Joker(1)));
    }
}
