use std::fmt;
use std::str::FromStr;

use crate::card::{Card, Rank, Suit};
use crate::state::Seat;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Token {
    V1,
    Cutthroat3P,
    Dealer,
    Deck,
    EndDeck,
    Unknown,
    P0,
    P1,
    P2,
    P3,
    Draw,
    Pass,
    Points,
    Scuttle,
    PlayRoyal,
    OneOff,
    Counter,
    Resolve,
    Discard,
    AC,
    AD,
    AH,
    AS,
    P2C,
    P2D,
    P2H,
    P2S,
    P3C,
    P3D,
    P3H,
    P3S,
    P4C,
    P4D,
    P4H,
    P4S,
    P5C,
    P5D,
    P5H,
    P5S,
    P6C,
    P6D,
    P6H,
    P6S,
    P7C,
    P7D,
    P7H,
    P7S,
    P8C,
    P8D,
    P8H,
    P8S,
    P9C,
    P9D,
    P9H,
    P9S,
    TC,
    TD,
    TH,
    TS,
    JC,
    JD,
    JH,
    JS,
    QC,
    QD,
    QH,
    QS,
    KC,
    KD,
    KH,
    KS,
    J0,
    J1,
}

impl Token {
    pub fn as_str(self) -> &'static str {
        match self {
            Token::V1 => "V1",
            Token::Cutthroat3P => "CUTTHROAT3P",
            Token::Dealer => "DEALER",
            Token::Deck => "DECK",
            Token::EndDeck => "ENDDECK",
            Token::Unknown => "UNKNOWN",
            Token::P0 => "P0",
            Token::P1 => "P1",
            Token::P2 => "P2",
            Token::P3 => "P3",
            Token::Draw => "draw",
            Token::Pass => "pass",
            Token::Points => "points",
            Token::Scuttle => "scuttle",
            Token::PlayRoyal => "playRoyal",
            Token::OneOff => "oneOff",
            Token::Counter => "counter",
            Token::Resolve => "resolve",
            Token::Discard => "discard",
            Token::AC => "AC",
            Token::AD => "AD",
            Token::AH => "AH",
            Token::AS => "AS",
            Token::P2C => "2C",
            Token::P2D => "2D",
            Token::P2H => "2H",
            Token::P2S => "2S",
            Token::P3C => "3C",
            Token::P3D => "3D",
            Token::P3H => "3H",
            Token::P3S => "3S",
            Token::P4C => "4C",
            Token::P4D => "4D",
            Token::P4H => "4H",
            Token::P4S => "4S",
            Token::P5C => "5C",
            Token::P5D => "5D",
            Token::P5H => "5H",
            Token::P5S => "5S",
            Token::P6C => "6C",
            Token::P6D => "6D",
            Token::P6H => "6H",
            Token::P6S => "6S",
            Token::P7C => "7C",
            Token::P7D => "7D",
            Token::P7H => "7H",
            Token::P7S => "7S",
            Token::P8C => "8C",
            Token::P8D => "8D",
            Token::P8H => "8H",
            Token::P8S => "8S",
            Token::P9C => "9C",
            Token::P9D => "9D",
            Token::P9H => "9H",
            Token::P9S => "9S",
            Token::TC => "TC",
            Token::TD => "TD",
            Token::TH => "TH",
            Token::TS => "TS",
            Token::JC => "JC",
            Token::JD => "JD",
            Token::JH => "JH",
            Token::JS => "JS",
            Token::QC => "QC",
            Token::QD => "QD",
            Token::QH => "QH",
            Token::QS => "QS",
            Token::KC => "KC",
            Token::KD => "KD",
            Token::KH => "KH",
            Token::KS => "KS",
            Token::J0 => "J0",
            Token::J1 => "J1",
        }
    }

    pub fn parse(token: &str) -> Option<Self> {
        Some(match token {
            "V1" => Token::V1,
            "CUTTHROAT3P" => Token::Cutthroat3P,
            "DEALER" => Token::Dealer,
            "DECK" => Token::Deck,
            "ENDDECK" => Token::EndDeck,
            "UNKNOWN" => Token::Unknown,
            "P0" => Token::P0,
            "P1" => Token::P1,
            "P2" => Token::P2,
            "P3" => Token::P3,
            "draw" => Token::Draw,
            "pass" => Token::Pass,
            "points" => Token::Points,
            "scuttle" => Token::Scuttle,
            "playRoyal" => Token::PlayRoyal,
            "oneOff" => Token::OneOff,
            "counter" => Token::Counter,
            "resolve" => Token::Resolve,
            "discard" => Token::Discard,
            "AC" => Token::AC,
            "AD" => Token::AD,
            "AH" => Token::AH,
            "AS" => Token::AS,
            "2C" => Token::P2C,
            "2D" => Token::P2D,
            "2H" => Token::P2H,
            "2S" => Token::P2S,
            "3C" => Token::P3C,
            "3D" => Token::P3D,
            "3H" => Token::P3H,
            "3S" => Token::P3S,
            "4C" => Token::P4C,
            "4D" => Token::P4D,
            "4H" => Token::P4H,
            "4S" => Token::P4S,
            "5C" => Token::P5C,
            "5D" => Token::P5D,
            "5H" => Token::P5H,
            "5S" => Token::P5S,
            "6C" => Token::P6C,
            "6D" => Token::P6D,
            "6H" => Token::P6H,
            "6S" => Token::P6S,
            "7C" => Token::P7C,
            "7D" => Token::P7D,
            "7H" => Token::P7H,
            "7S" => Token::P7S,
            "8C" => Token::P8C,
            "8D" => Token::P8D,
            "8H" => Token::P8H,
            "8S" => Token::P8S,
            "9C" => Token::P9C,
            "9D" => Token::P9D,
            "9H" => Token::P9H,
            "9S" => Token::P9S,
            "TC" => Token::TC,
            "TD" => Token::TD,
            "TH" => Token::TH,
            "TS" => Token::TS,
            "JC" => Token::JC,
            "JD" => Token::JD,
            "JH" => Token::JH,
            "JS" => Token::JS,
            "QC" => Token::QC,
            "QD" => Token::QD,
            "QH" => Token::QH,
            "QS" => Token::QS,
            "KC" => Token::KC,
            "KD" => Token::KD,
            "KH" => Token::KH,
            "KS" => Token::KS,
            "J0" => Token::J0,
            "J1" => Token::J1,
            _ => return None,
        })
    }

    pub fn is_verb(self) -> bool {
        matches!(
            self,
            Token::Draw
                | Token::Pass
                | Token::Points
                | Token::Scuttle
                | Token::PlayRoyal
                | Token::OneOff
                | Token::Counter
                | Token::Resolve
                | Token::Discard
        )
    }

    pub fn is_seat(self) -> bool {
        matches!(self, Token::P0 | Token::P1 | Token::P2 | Token::P3)
    }

    pub fn seat(self) -> Option<Seat> {
        Some(match self {
            Token::P0 => 0,
            Token::P1 => 1,
            Token::P2 => 2,
            Token::P3 => 3,
            _ => return None,
        })
    }

    pub fn from_seat(seat: Seat) -> Option<Self> {
        match seat {
            0 => Some(Token::P0),
            1 => Some(Token::P1),
            2 => Some(Token::P2),
            3 => Some(Token::P3),
            _ => None,
        }
    }

    pub fn card(self) -> Option<Card> {
        Some(match self {
            Token::J0 => Card::Joker(0),
            Token::J1 => Card::Joker(1),
            Token::AC => Card::Standard {
                rank: Rank::Ace,
                suit: Suit::Clubs,
            },
            Token::AD => Card::Standard {
                rank: Rank::Ace,
                suit: Suit::Diamonds,
            },
            Token::AH => Card::Standard {
                rank: Rank::Ace,
                suit: Suit::Hearts,
            },
            Token::AS => Card::Standard {
                rank: Rank::Ace,
                suit: Suit::Spades,
            },
            Token::P2C => Card::Standard {
                rank: Rank::Two,
                suit: Suit::Clubs,
            },
            Token::P2D => Card::Standard {
                rank: Rank::Two,
                suit: Suit::Diamonds,
            },
            Token::P2H => Card::Standard {
                rank: Rank::Two,
                suit: Suit::Hearts,
            },
            Token::P2S => Card::Standard {
                rank: Rank::Two,
                suit: Suit::Spades,
            },
            Token::P3C => Card::Standard {
                rank: Rank::Three,
                suit: Suit::Clubs,
            },
            Token::P3D => Card::Standard {
                rank: Rank::Three,
                suit: Suit::Diamonds,
            },
            Token::P3H => Card::Standard {
                rank: Rank::Three,
                suit: Suit::Hearts,
            },
            Token::P3S => Card::Standard {
                rank: Rank::Three,
                suit: Suit::Spades,
            },
            Token::P4C => Card::Standard {
                rank: Rank::Four,
                suit: Suit::Clubs,
            },
            Token::P4D => Card::Standard {
                rank: Rank::Four,
                suit: Suit::Diamonds,
            },
            Token::P4H => Card::Standard {
                rank: Rank::Four,
                suit: Suit::Hearts,
            },
            Token::P4S => Card::Standard {
                rank: Rank::Four,
                suit: Suit::Spades,
            },
            Token::P5C => Card::Standard {
                rank: Rank::Five,
                suit: Suit::Clubs,
            },
            Token::P5D => Card::Standard {
                rank: Rank::Five,
                suit: Suit::Diamonds,
            },
            Token::P5H => Card::Standard {
                rank: Rank::Five,
                suit: Suit::Hearts,
            },
            Token::P5S => Card::Standard {
                rank: Rank::Five,
                suit: Suit::Spades,
            },
            Token::P6C => Card::Standard {
                rank: Rank::Six,
                suit: Suit::Clubs,
            },
            Token::P6D => Card::Standard {
                rank: Rank::Six,
                suit: Suit::Diamonds,
            },
            Token::P6H => Card::Standard {
                rank: Rank::Six,
                suit: Suit::Hearts,
            },
            Token::P6S => Card::Standard {
                rank: Rank::Six,
                suit: Suit::Spades,
            },
            Token::P7C => Card::Standard {
                rank: Rank::Seven,
                suit: Suit::Clubs,
            },
            Token::P7D => Card::Standard {
                rank: Rank::Seven,
                suit: Suit::Diamonds,
            },
            Token::P7H => Card::Standard {
                rank: Rank::Seven,
                suit: Suit::Hearts,
            },
            Token::P7S => Card::Standard {
                rank: Rank::Seven,
                suit: Suit::Spades,
            },
            Token::P8C => Card::Standard {
                rank: Rank::Eight,
                suit: Suit::Clubs,
            },
            Token::P8D => Card::Standard {
                rank: Rank::Eight,
                suit: Suit::Diamonds,
            },
            Token::P8H => Card::Standard {
                rank: Rank::Eight,
                suit: Suit::Hearts,
            },
            Token::P8S => Card::Standard {
                rank: Rank::Eight,
                suit: Suit::Spades,
            },
            Token::P9C => Card::Standard {
                rank: Rank::Nine,
                suit: Suit::Clubs,
            },
            Token::P9D => Card::Standard {
                rank: Rank::Nine,
                suit: Suit::Diamonds,
            },
            Token::P9H => Card::Standard {
                rank: Rank::Nine,
                suit: Suit::Hearts,
            },
            Token::P9S => Card::Standard {
                rank: Rank::Nine,
                suit: Suit::Spades,
            },
            Token::TC => Card::Standard {
                rank: Rank::Ten,
                suit: Suit::Clubs,
            },
            Token::TD => Card::Standard {
                rank: Rank::Ten,
                suit: Suit::Diamonds,
            },
            Token::TH => Card::Standard {
                rank: Rank::Ten,
                suit: Suit::Hearts,
            },
            Token::TS => Card::Standard {
                rank: Rank::Ten,
                suit: Suit::Spades,
            },
            Token::JC => Card::Standard {
                rank: Rank::Jack,
                suit: Suit::Clubs,
            },
            Token::JD => Card::Standard {
                rank: Rank::Jack,
                suit: Suit::Diamonds,
            },
            Token::JH => Card::Standard {
                rank: Rank::Jack,
                suit: Suit::Hearts,
            },
            Token::JS => Card::Standard {
                rank: Rank::Jack,
                suit: Suit::Spades,
            },
            Token::QC => Card::Standard {
                rank: Rank::Queen,
                suit: Suit::Clubs,
            },
            Token::QD => Card::Standard {
                rank: Rank::Queen,
                suit: Suit::Diamonds,
            },
            Token::QH => Card::Standard {
                rank: Rank::Queen,
                suit: Suit::Hearts,
            },
            Token::QS => Card::Standard {
                rank: Rank::Queen,
                suit: Suit::Spades,
            },
            Token::KC => Card::Standard {
                rank: Rank::King,
                suit: Suit::Clubs,
            },
            Token::KD => Card::Standard {
                rank: Rank::King,
                suit: Suit::Diamonds,
            },
            Token::KH => Card::Standard {
                rank: Rank::King,
                suit: Suit::Hearts,
            },
            Token::KS => Card::Standard {
                rank: Rank::King,
                suit: Suit::Spades,
            },
            _ => return None,
        })
    }

    pub fn from_card(card: Card) -> Token {
        match card {
            Card::Joker(0) => Token::J0,
            Card::Joker(1) => Token::J1,
            Card::Joker(_) => Token::J0,
            Card::Standard {
                rank: Rank::Ace,
                suit: Suit::Clubs,
            } => Token::AC,
            Card::Standard {
                rank: Rank::Ace,
                suit: Suit::Diamonds,
            } => Token::AD,
            Card::Standard {
                rank: Rank::Ace,
                suit: Suit::Hearts,
            } => Token::AH,
            Card::Standard {
                rank: Rank::Ace,
                suit: Suit::Spades,
            } => Token::AS,
            Card::Standard {
                rank: Rank::Two,
                suit: Suit::Clubs,
            } => Token::P2C,
            Card::Standard {
                rank: Rank::Two,
                suit: Suit::Diamonds,
            } => Token::P2D,
            Card::Standard {
                rank: Rank::Two,
                suit: Suit::Hearts,
            } => Token::P2H,
            Card::Standard {
                rank: Rank::Two,
                suit: Suit::Spades,
            } => Token::P2S,
            Card::Standard {
                rank: Rank::Three,
                suit: Suit::Clubs,
            } => Token::P3C,
            Card::Standard {
                rank: Rank::Three,
                suit: Suit::Diamonds,
            } => Token::P3D,
            Card::Standard {
                rank: Rank::Three,
                suit: Suit::Hearts,
            } => Token::P3H,
            Card::Standard {
                rank: Rank::Three,
                suit: Suit::Spades,
            } => Token::P3S,
            Card::Standard {
                rank: Rank::Four,
                suit: Suit::Clubs,
            } => Token::P4C,
            Card::Standard {
                rank: Rank::Four,
                suit: Suit::Diamonds,
            } => Token::P4D,
            Card::Standard {
                rank: Rank::Four,
                suit: Suit::Hearts,
            } => Token::P4H,
            Card::Standard {
                rank: Rank::Four,
                suit: Suit::Spades,
            } => Token::P4S,
            Card::Standard {
                rank: Rank::Five,
                suit: Suit::Clubs,
            } => Token::P5C,
            Card::Standard {
                rank: Rank::Five,
                suit: Suit::Diamonds,
            } => Token::P5D,
            Card::Standard {
                rank: Rank::Five,
                suit: Suit::Hearts,
            } => Token::P5H,
            Card::Standard {
                rank: Rank::Five,
                suit: Suit::Spades,
            } => Token::P5S,
            Card::Standard {
                rank: Rank::Six,
                suit: Suit::Clubs,
            } => Token::P6C,
            Card::Standard {
                rank: Rank::Six,
                suit: Suit::Diamonds,
            } => Token::P6D,
            Card::Standard {
                rank: Rank::Six,
                suit: Suit::Hearts,
            } => Token::P6H,
            Card::Standard {
                rank: Rank::Six,
                suit: Suit::Spades,
            } => Token::P6S,
            Card::Standard {
                rank: Rank::Seven,
                suit: Suit::Clubs,
            } => Token::P7C,
            Card::Standard {
                rank: Rank::Seven,
                suit: Suit::Diamonds,
            } => Token::P7D,
            Card::Standard {
                rank: Rank::Seven,
                suit: Suit::Hearts,
            } => Token::P7H,
            Card::Standard {
                rank: Rank::Seven,
                suit: Suit::Spades,
            } => Token::P7S,
            Card::Standard {
                rank: Rank::Eight,
                suit: Suit::Clubs,
            } => Token::P8C,
            Card::Standard {
                rank: Rank::Eight,
                suit: Suit::Diamonds,
            } => Token::P8D,
            Card::Standard {
                rank: Rank::Eight,
                suit: Suit::Hearts,
            } => Token::P8H,
            Card::Standard {
                rank: Rank::Eight,
                suit: Suit::Spades,
            } => Token::P8S,
            Card::Standard {
                rank: Rank::Nine,
                suit: Suit::Clubs,
            } => Token::P9C,
            Card::Standard {
                rank: Rank::Nine,
                suit: Suit::Diamonds,
            } => Token::P9D,
            Card::Standard {
                rank: Rank::Nine,
                suit: Suit::Hearts,
            } => Token::P9H,
            Card::Standard {
                rank: Rank::Nine,
                suit: Suit::Spades,
            } => Token::P9S,
            Card::Standard {
                rank: Rank::Ten,
                suit: Suit::Clubs,
            } => Token::TC,
            Card::Standard {
                rank: Rank::Ten,
                suit: Suit::Diamonds,
            } => Token::TD,
            Card::Standard {
                rank: Rank::Ten,
                suit: Suit::Hearts,
            } => Token::TH,
            Card::Standard {
                rank: Rank::Ten,
                suit: Suit::Spades,
            } => Token::TS,
            Card::Standard {
                rank: Rank::Jack,
                suit: Suit::Clubs,
            } => Token::JC,
            Card::Standard {
                rank: Rank::Jack,
                suit: Suit::Diamonds,
            } => Token::JD,
            Card::Standard {
                rank: Rank::Jack,
                suit: Suit::Hearts,
            } => Token::JH,
            Card::Standard {
                rank: Rank::Jack,
                suit: Suit::Spades,
            } => Token::JS,
            Card::Standard {
                rank: Rank::Queen,
                suit: Suit::Clubs,
            } => Token::QC,
            Card::Standard {
                rank: Rank::Queen,
                suit: Suit::Diamonds,
            } => Token::QD,
            Card::Standard {
                rank: Rank::Queen,
                suit: Suit::Hearts,
            } => Token::QH,
            Card::Standard {
                rank: Rank::Queen,
                suit: Suit::Spades,
            } => Token::QS,
            Card::Standard {
                rank: Rank::King,
                suit: Suit::Clubs,
            } => Token::KC,
            Card::Standard {
                rank: Rank::King,
                suit: Suit::Diamonds,
            } => Token::KD,
            Card::Standard {
                rank: Rank::King,
                suit: Suit::Hearts,
            } => Token::KH,
            Card::Standard {
                rank: Rank::King,
                suit: Suit::Spades,
            } => Token::KS,
        }
    }
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Token {
    type Err = ();

    fn from_str(token: &str) -> Result<Self, Self::Err> {
        Token::parse(token).ok_or(())
    }
}

pub fn join_tokens(tokens: &[Token]) -> String {
    let mut out = String::new();
    for (idx, token) in tokens.iter().enumerate() {
        if idx > 0 {
            out.push(' ');
        }
        out.push_str(token.as_str());
    }
    out
}

pub fn parse_token_slice(input: &str) -> Option<Vec<Token>> {
    let mut out = Vec::new();
    for part in input.split_whitespace() {
        out.push(part.parse::<Token>().ok()?);
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::Token;
    use crate::card::full_deck_with_jokers;

    #[test]
    fn token_string_roundtrip_covers_full_vocab() {
        let all = [
            "V1",
            "CUTTHROAT3P",
            "DEALER",
            "DECK",
            "ENDDECK",
            "UNKNOWN",
            "P0",
            "P1",
            "P2",
            "P3",
            "draw",
            "pass",
            "points",
            "scuttle",
            "playRoyal",
            "oneOff",
            "counter",
            "resolve",
            "discard",
            "AC",
            "AD",
            "AH",
            "AS",
            "2C",
            "2D",
            "2H",
            "2S",
            "3C",
            "3D",
            "3H",
            "3S",
            "4C",
            "4D",
            "4H",
            "4S",
            "5C",
            "5D",
            "5H",
            "5S",
            "6C",
            "6D",
            "6H",
            "6S",
            "7C",
            "7D",
            "7H",
            "7S",
            "8C",
            "8D",
            "8H",
            "8S",
            "9C",
            "9D",
            "9H",
            "9S",
            "TC",
            "TD",
            "TH",
            "TS",
            "JC",
            "JD",
            "JH",
            "JS",
            "QC",
            "QD",
            "QH",
            "QS",
            "KC",
            "KD",
            "KH",
            "KS",
            "J0",
            "J1",
        ];
        for raw in all {
            let token = raw.parse::<Token>().expect("token should parse");
            assert_eq!(token.as_str(), raw);
        }
    }

    #[test]
    fn card_token_mapping_is_bijective_for_deck() {
        let mut deck = full_deck_with_jokers();
        deck.sort_by_key(|card| card.to_token());
        for card in deck {
            let token = Token::from_card(card);
            assert_eq!(token.card(), Some(card));
        }
    }
}
