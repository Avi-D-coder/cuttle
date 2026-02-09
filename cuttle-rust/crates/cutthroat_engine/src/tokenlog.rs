use crate::action::{Action, OneOffTarget, SevenPlay};
use crate::card::Card;
use crate::state::{CutthroatState, PLAYER_COUNT, RuleError, Seat};
use thiserror::Error;

const VERSION: &str = "V1";
const MODE: &str = "CUTTHROAT3P";

#[derive(Clone, Debug)]
pub struct TokenLog {
    pub dealer: Seat,
    pub deck: Vec<Card>,
    pub actions: Vec<(Seat, Action)>,
}

#[derive(Clone, Debug, Error)]
pub enum TokenError {
    #[error("invalid token stream")]
    InvalidFormat,
    #[error("unknown card token")]
    UnknownCard,
    #[error("unknown action token")]
    UnknownAction,
    #[error("replay error: {0}")]
    Replay(#[from] RuleError),
}

#[derive(Clone, Debug, Error)]
pub enum TokenizeError {
    #[error("invalid seat")]
    InvalidSeat,
}

pub fn encode_header(dealer: Seat, deck: &[Card]) -> String {
    let mut tokens = vec![
        VERSION.to_string(),
        MODE.to_string(),
        "DEALER".to_string(),
        seat_to_token(dealer),
        "DECK".to_string(),
    ];
    for card in deck {
        tokens.push(card.to_token());
    }
    tokens.push("ENDDECK".to_string());
    tokens.join(" ")
}

pub fn parse(tokens: &str) -> Result<TokenLog, TokenError> {
    let mut parts = tokens.split_whitespace().peekable();
    if parts.next() != Some(VERSION) || parts.next() != Some(MODE) {
        return Err(TokenError::InvalidFormat);
    }
    if parts.next() != Some("DEALER") {
        return Err(TokenError::InvalidFormat);
    }
    let dealer_token = parts.next().ok_or(TokenError::InvalidFormat)?;
    let dealer = parse_seat(dealer_token)?;
    if parts.next() != Some("DECK") {
        return Err(TokenError::InvalidFormat);
    }
    let mut deck = Vec::new();
    loop {
        let tok = parts.next().ok_or(TokenError::InvalidFormat)?;
        if tok == "ENDDECK" {
            break;
        }
        let card = Card::from_token(tok).ok_or(TokenError::UnknownCard)?;
        deck.push(card);
    }

    let mut actions = Vec::new();
    while let Some(seat_tok) = parts.next() {
        let seat = parse_seat(seat_tok)?;
        let action = parse_action(&mut parts)?;
        actions.push((seat, action));
    }

    Ok(TokenLog {
        dealer,
        deck,
        actions,
    })
}

pub fn replay(log: &TokenLog) -> Result<CutthroatState, TokenError> {
    let required_cards = (PLAYER_COUNT as usize) * 5;
    if log.deck.len() < required_cards {
        return Err(TokenError::InvalidFormat);
    }
    let mut state = CutthroatState::new_with_deck(log.dealer, log.deck.clone());
    for (seat, action) in &log.actions {
        state.apply(*seat, action.clone())?;
    }
    Ok(state)
}

pub fn append_action(
    tokens: &mut String,
    seat: Seat,
    action: &Action,
) -> Result<(), TokenizeError> {
    if seat >= PLAYER_COUNT {
        return Err(TokenizeError::InvalidSeat);
    }
    if !tokens.is_empty() {
        tokens.push(' ');
    }
    tokens.push_str(&seat_to_token(seat));
    tokens.push(' ');
    let action_tokens = encode_action(action);
    tokens.push_str(&action_tokens.join(" "));
    Ok(())
}

fn encode_action(action: &Action) -> Vec<String> {
    match action {
        Action::Draw => vec!["MT_DRAW".to_string()],
        Action::Pass => vec!["MT_PASS".to_string()],
        Action::PlayPoints { card } => vec!["MT_POINTS".to_string(), card.to_token()],
        Action::Scuttle {
            card,
            target_point_base,
        } => vec![
            "MT_SCUTTLE".to_string(),
            card.to_token(),
            "TGT".to_string(),
            target_point_base.to_token(),
        ],
        Action::PlayRoyal { card } => vec!["MT_ROYAL".to_string(), card.to_token()],
        Action::PlayJack {
            jack,
            target_point_base,
        } => vec![
            "MT_JACK".to_string(),
            jack.to_token(),
            "TGT".to_string(),
            target_point_base.to_token(),
        ],
        Action::PlayJoker {
            joker,
            target_royal_card,
        } => vec![
            "MT_JOKER".to_string(),
            joker.to_token(),
            "TGT".to_string(),
            target_royal_card.to_token(),
        ],
        Action::PlayOneOff { card, target } => {
            let mut tokens = vec!["MT_ONEOFF".to_string(), card.to_token()];
            encode_oneoff_target(target, &mut tokens);
            tokens
        }
        Action::CounterTwo { two_card } => vec!["MT_C2".to_string(), two_card.to_token()],
        Action::CounterPass => vec!["MT_CPASS".to_string()],
        Action::ResolveThreePick { card_from_scrap } => {
            vec!["MT_R3_PICK".to_string(), card_from_scrap.to_token()]
        }
        Action::ResolveFourDiscard { card } => vec!["MT_R4_DISCARD".to_string(), card.to_token()],
        Action::ResolveFiveDiscard { card } => vec!["MT_R5_DISCARD".to_string(), card.to_token()],
        Action::ResolveSevenChoose { source_index, play } => {
            let mut tokens = vec![
                "MT_R7".to_string(),
                "SRC".to_string(),
                source_index.to_string(),
                "AS".to_string(),
            ];
            match play {
                SevenPlay::Points => tokens.push("POINTS".to_string()),
                SevenPlay::Scuttle { target } => {
                    tokens.push("SCUTTLE".to_string());
                    tokens.push(target.to_token());
                }
                SevenPlay::Royal => tokens.push("ROYAL".to_string()),
                SevenPlay::Jack { target } => {
                    tokens.push("JACK".to_string());
                    tokens.push(target.to_token());
                }
                SevenPlay::Joker { target } => {
                    tokens.push("JOKER".to_string());
                    tokens.push(target.to_token());
                }
                SevenPlay::OneOff { target } => {
                    tokens.push("ONEOFF".to_string());
                    encode_oneoff_target(target, &mut tokens);
                }
                SevenPlay::Discard => tokens.push("DISCARD".to_string()),
            }
            tokens
        }
    }
}

fn parse_action<'a, I>(parts: &mut std::iter::Peekable<I>) -> Result<Action, TokenError>
where
    I: Iterator<Item = &'a str>,
{
    let tok = parts.next().ok_or(TokenError::InvalidFormat)?;
    match tok {
        "MT_DRAW" => Ok(Action::Draw),
        "MT_PASS" => Ok(Action::Pass),
        "MT_POINTS" => {
            let card = parse_card(parts.next())?;
            Ok(Action::PlayPoints { card })
        }
        "MT_SCUTTLE" => {
            let card = parse_card(parts.next())?;
            expect(parts.next(), "TGT")?;
            let target = parse_card(parts.next())?;
            Ok(Action::Scuttle {
                card,
                target_point_base: target,
            })
        }
        "MT_ROYAL" => {
            let card = parse_card(parts.next())?;
            Ok(Action::PlayRoyal { card })
        }
        "MT_JACK" => {
            let card = parse_card(parts.next())?;
            expect(parts.next(), "TGT")?;
            let target = parse_card(parts.next())?;
            Ok(Action::PlayJack {
                jack: card,
                target_point_base: target,
            })
        }
        "MT_JOKER" => {
            let card = parse_card(parts.next())?;
            expect(parts.next(), "TGT")?;
            let target = parse_card(parts.next())?;
            Ok(Action::PlayJoker {
                joker: card,
                target_royal_card: target,
            })
        }
        "MT_ONEOFF" => {
            let card = parse_card(parts.next())?;
            let target = parse_oneoff_target(parts)?;
            Ok(Action::PlayOneOff { card, target })
        }
        "MT_C2" => {
            let card = parse_card(parts.next())?;
            Ok(Action::CounterTwo { two_card: card })
        }
        "MT_CPASS" => Ok(Action::CounterPass),
        "MT_R3_PICK" => {
            let card = parse_card(parts.next())?;
            Ok(Action::ResolveThreePick {
                card_from_scrap: card,
            })
        }
        "MT_R4_DISCARD" => {
            let card = parse_card(parts.next())?;
            Ok(Action::ResolveFourDiscard { card })
        }
        "MT_R5_DISCARD" => {
            let card = parse_card(parts.next())?;
            Ok(Action::ResolveFiveDiscard { card })
        }
        "MT_R7" => {
            expect(parts.next(), "SRC")?;
            let idx = parts
                .next()
                .ok_or(TokenError::InvalidFormat)?
                .parse::<u8>()
                .map_err(|_| TokenError::InvalidFormat)?;
            expect(parts.next(), "AS")?;
            let play_tok = parts.next().ok_or(TokenError::InvalidFormat)?;
            let play = match play_tok {
                "POINTS" => SevenPlay::Points,
                "SCUTTLE" => SevenPlay::Scuttle {
                    target: parse_card(parts.next())?,
                },
                "ROYAL" => SevenPlay::Royal,
                "JACK" => SevenPlay::Jack {
                    target: parse_card(parts.next())?,
                },
                "JOKER" => SevenPlay::Joker {
                    target: parse_card(parts.next())?,
                },
                "ONEOFF" => SevenPlay::OneOff {
                    target: parse_oneoff_target(parts)?,
                },
                "DISCARD" => SevenPlay::Discard,
                _ => return Err(TokenError::UnknownAction),
            };
            Ok(Action::ResolveSevenChoose {
                source_index: idx,
                play,
            })
        }
        _ => Err(TokenError::UnknownAction),
    }
}

fn encode_oneoff_target(target: &OneOffTarget, tokens: &mut Vec<String>) {
    match target {
        OneOffTarget::None => {}
        OneOffTarget::Player { seat } => {
            tokens.push("TGT_P".to_string());
            tokens.push(seat_to_token(*seat));
        }
        OneOffTarget::Point { base } => {
            tokens.push("TGT_POINT".to_string());
            tokens.push(base.to_token());
        }
        OneOffTarget::Royal { card } => {
            tokens.push("TGT_ROYAL".to_string());
            tokens.push(card.to_token());
        }
        OneOffTarget::Jack { card } => {
            tokens.push("TGT_JACK".to_string());
            tokens.push(card.to_token());
        }
        OneOffTarget::Joker { card } => {
            tokens.push("TGT_JOKER".to_string());
            tokens.push(card.to_token());
        }
    }
}

fn parse_oneoff_target<'a, I>(
    parts: &mut std::iter::Peekable<I>,
) -> Result<OneOffTarget, TokenError>
where
    I: Iterator<Item = &'a str>,
{
    let Some(tok) = parts.peek().copied() else {
        return Ok(OneOffTarget::None);
    };
    if !tok.starts_with("TGT_") {
        return Ok(OneOffTarget::None);
    }
    let tok = parts.next().ok_or(TokenError::InvalidFormat)?;
    match tok {
        "TGT_P" => {
            let seat = parse_seat(parts.next().ok_or(TokenError::InvalidFormat)?)?;
            Ok(OneOffTarget::Player { seat })
        }
        "TGT_POINT" => Ok(OneOffTarget::Point {
            base: parse_card(parts.next())?,
        }),
        "TGT_ROYAL" => Ok(OneOffTarget::Royal {
            card: parse_card(parts.next())?,
        }),
        "TGT_JACK" => Ok(OneOffTarget::Jack {
            card: parse_card(parts.next())?,
        }),
        "TGT_JOKER" => Ok(OneOffTarget::Joker {
            card: parse_card(parts.next())?,
        }),
        _ => Err(TokenError::InvalidFormat),
    }
}

fn parse_card(token: Option<&str>) -> Result<Card, TokenError> {
    let Some(tok) = token else {
        return Err(TokenError::InvalidFormat);
    };
    Card::from_token(tok).ok_or(TokenError::UnknownCard)
}

fn parse_seat(token: &str) -> Result<Seat, TokenError> {
    if token.len() != 2 || !token.starts_with('P') {
        return Err(TokenError::InvalidFormat);
    }
    let num = token[1..]
        .parse::<u8>()
        .map_err(|_| TokenError::InvalidFormat)?;
    if num >= PLAYER_COUNT {
        return Err(TokenError::InvalidFormat);
    }
    Ok(num)
}

fn seat_to_token(seat: Seat) -> String {
    format!("P{}", seat)
}

fn expect(actual: Option<&str>, expected: &str) -> Result<(), TokenError> {
    if actual == Some(expected) {
        Ok(())
    } else {
        Err(TokenError::InvalidFormat)
    }
}

#[cfg(test)]
mod tests {
    use super::{TokenError, TokenLog, parse, replay};
    use crate::action::Action;

    #[test]
    fn parse_rejects_out_of_range_seat_token() {
        let tokenlog = "V1 CUTTHROAT3P DEALER P3 DECK ENDDECK";
        let err = parse(tokenlog).expect_err("seat >= player count should be rejected");
        assert!(matches!(err, TokenError::InvalidFormat));
    }

    #[test]
    fn replay_rejects_short_deck_without_panicking() {
        let log = TokenLog {
            dealer: 0,
            deck: Vec::new(),
            actions: vec![(0, Action::Pass)],
        };
        let err = replay(&log).expect_err("short deck should be rejected");
        assert!(matches!(err, TokenError::InvalidFormat));
    }
}
