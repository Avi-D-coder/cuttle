use crate::action::{Action, OneOffTarget, SevenPlay};
use crate::card::{Card, Rank};
use crate::state::{CutthroatState, PLAYER_COUNT, Phase, RuleError, Seat};
use crate::tokens::{Token, join_tokens, parse_token_slice};
use thiserror::Error;

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
    #[error("invalid action context")]
    InvalidActionContext,
}

pub fn encode_header(dealer: Seat, deck: &[Card]) -> String {
    let dealer_tok = Token::from_seat(dealer).unwrap_or(Token::P0);
    let mut tokens = vec![
        Token::V1,
        Token::Cutthroat3P,
        Token::Dealer,
        dealer_tok,
        Token::Deck,
    ];
    for card in deck {
        tokens.push(card.to_token_enum());
    }
    tokens.push(Token::EndDeck);
    join_tokens(&tokens)
}

pub fn parse(tokens: &str) -> Result<TokenLog, TokenError> {
    let parts = parse_token_slice(tokens).ok_or(TokenError::InvalidFormat)?;
    let mut cursor = 0usize;

    if parts.get(cursor).copied() != Some(Token::V1)
        || parts.get(cursor + 1).copied() != Some(Token::Cutthroat3P)
    {
        return Err(TokenError::InvalidFormat);
    }
    cursor += 2;
    if parts.get(cursor).copied() != Some(Token::Dealer) {
        return Err(TokenError::InvalidFormat);
    }
    cursor += 1;
    let dealer_token = parts
        .get(cursor)
        .copied()
        .ok_or(TokenError::InvalidFormat)?;
    let dealer = parse_seat_token(dealer_token)?;
    cursor += 1;
    if parts.get(cursor).copied() != Some(Token::Deck) {
        return Err(TokenError::InvalidFormat);
    }
    cursor += 1;

    let mut deck = Vec::new();
    while cursor < parts.len() {
        let tok = parts[cursor];
        cursor += 1;
        if tok == Token::EndDeck {
            break;
        }
        let card = tok.card().ok_or(TokenError::UnknownCard)?;
        deck.push(card);
    }
    if cursor == parts.len() && parts.last().copied() != Some(Token::EndDeck) {
        return Err(TokenError::InvalidFormat);
    }

    let required_cards = (PLAYER_COUNT as usize) * 5;
    if deck.len() < required_cards {
        return Err(TokenError::InvalidFormat);
    }

    let mut state = CutthroatState::new_with_deck(dealer, deck.clone());

    let mut actions = Vec::new();
    while cursor < parts.len() {
        let seat_tok = parts
            .get(cursor)
            .copied()
            .ok_or(TokenError::InvalidFormat)?;
        cursor += 1;
        let seat = parse_seat_token(seat_tok)?;
        let (action, next_cursor) = parse_action_with_state(&parts, cursor, &state)?;
        cursor = next_cursor;
        let legal = state.legal_actions(seat);
        if !legal.contains(&action) {
            return Err(TokenError::InvalidFormat);
        }
        state
            .apply(seat, action.clone())
            .map_err(TokenError::Replay)?;
        actions.push((seat, action));
    }

    Ok(TokenLog {
        dealer,
        deck,
        actions,
    })
}

pub fn parse_action_tokens_for_state(
    action_tokens: &str,
    state: &CutthroatState,
) -> Result<(Seat, Action), TokenError> {
    let mut parts = parse_token_slice(action_tokens).ok_or(TokenError::InvalidFormat)?;
    parse_action_token_stream_for_state(&mut parts, state)
}

pub fn parse_action_token_stream_for_state(
    parts: &mut Vec<Token>,
    state: &CutthroatState,
) -> Result<(Seat, Action), TokenError> {
    if parts.is_empty() {
        return Err(TokenError::InvalidFormat);
    }
    let seat = parse_seat_token(parts[0])?;
    normalize_action_tokens_for_state(parts, state)?;
    let (action, next_cursor) = parse_action_with_state(parts, 1, state)?;
    if next_cursor != parts.len() {
        return Err(TokenError::InvalidFormat);
    }
    Ok((seat, action))
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
    state_before: &CutthroatState,
    seat: Seat,
    action: &Action,
) -> Result<(), TokenizeError> {
    if seat >= PLAYER_COUNT {
        return Err(TokenizeError::InvalidSeat);
    }
    if !tokens.is_empty() {
        tokens.push(' ');
    }
    let mut action_tokens = vec![seat_to_token(seat)?];
    action_tokens.extend(encode_action(action, state_before, seat, true)?);
    tokens.push_str(&join_tokens(&action_tokens));
    Ok(())
}

pub fn encode_action_tokens_for_input(
    state_before: &CutthroatState,
    seat: Seat,
    action: &Action,
) -> Result<String, TokenizeError> {
    Ok(join_tokens(&encode_action_token_vec_for_input(
        state_before,
        seat,
        action,
    )?))
}

pub fn encode_action_token_vec_for_input(
    state_before: &CutthroatState,
    seat: Seat,
    action: &Action,
) -> Result<Vec<Token>, TokenizeError> {
    if seat >= PLAYER_COUNT {
        return Err(TokenizeError::InvalidSeat);
    }
    let mut tokens = vec![seat_to_token(seat)?];
    tokens.extend(encode_action(action, state_before, seat, false)?);
    Ok(tokens)
}

fn encode_action(
    action: &Action,
    state_before: &CutthroatState,
    actor_seat: Seat,
    include_glasses_snapshot: bool,
) -> Result<Vec<Token>, TokenizeError> {
    let mut tokens = match action {
        Action::Draw => {
            let card = state_before
                .deck
                .first()
                .copied()
                .ok_or(TokenizeError::InvalidActionContext)?;
            vec![Token::Draw, card.to_token_enum()]
        }
        Action::Pass => vec![Token::Pass],
        Action::PlayPoints { card } => vec![Token::Points, card.to_token_enum()],
        Action::Scuttle {
            card,
            target_point_base,
        } => vec![
            Token::Scuttle,
            card.to_token_enum(),
            target_point_base.to_token_enum(),
        ],
        Action::PlayRoyal { card } => vec![Token::PlayRoyal, card.to_token_enum()],
        Action::PlayJack {
            jack,
            target_point_base,
        } => vec![
            Token::PlayRoyal,
            jack.to_token_enum(),
            target_point_base.to_token_enum(),
        ],
        Action::PlayJoker {
            joker,
            target_royal_card,
        } => vec![
            Token::PlayRoyal,
            joker.to_token_enum(),
            target_royal_card.to_token_enum(),
        ],
        Action::PlayOneOff { card, target } => {
            let mut out = vec![Token::OneOff, card.to_token_enum()];
            encode_inline_oneoff_target(target, &mut out);
            out
        }
        Action::CounterTwo { two_card } => vec![Token::Counter, two_card.to_token_enum()],
        Action::CounterPass => vec![Token::Resolve],
        Action::ResolveThreePick { card_from_scrap } => {
            vec![Token::Resolve, card_from_scrap.to_token_enum()]
        }
        Action::ResolveFourDiscard { card } => {
            vec![Token::Resolve, Token::Discard, card.to_token_enum()]
        }
        Action::ResolveFiveDiscard { card } => vec![Token::Discard, card.to_token_enum()],
        Action::ResolveSevenChoose { card, play } => encode_resolving_seven_action(*card, play)?,
    };

    let royal_card = match action {
        Action::PlayRoyal { card } => Some(*card),
        Action::ResolveSevenChoose {
            card,
            play: SevenPlay::Royal,
        } => Some(*card),
        _ => None,
    };
    if include_glasses_snapshot
        && let Some(card) = royal_card
        && is_glasses_card(card)
    {
        append_glasses_snapshot_tokens(&mut tokens, state_before, actor_seat)?;
    }

    Ok(tokens)
}

fn encode_resolving_seven_action(
    chosen: Card,
    play: &SevenPlay,
) -> Result<Vec<Token>, TokenizeError> {
    let chosen_token = chosen.to_token_enum();
    let tokens = match play {
        SevenPlay::Points => vec![Token::Points, chosen_token],
        SevenPlay::Scuttle { target } => vec![Token::Scuttle, chosen_token, target.to_token_enum()],
        SevenPlay::Royal => vec![Token::PlayRoyal, chosen_token],
        SevenPlay::Jack { target } | SevenPlay::Joker { target } => {
            vec![Token::PlayRoyal, chosen_token, target.to_token_enum()]
        }
        SevenPlay::OneOff { target } => {
            let mut out = vec![Token::OneOff, chosen_token];
            encode_inline_oneoff_target(target, &mut out);
            out
        }
        SevenPlay::Discard => vec![Token::Discard, chosen_token],
    };
    Ok(tokens)
}

fn normalize_action_tokens_for_state(
    tokens: &mut Vec<Token>,
    state: &CutthroatState,
) -> Result<(), TokenError> {
    let Some(verb) = tokens.get(1).copied() else {
        return Err(TokenError::InvalidFormat);
    };

    if verb == Token::Draw && tokens.len() == 2 {
        let card = state
            .deck
            .first()
            .copied()
            .ok_or(TokenError::InvalidFormat)?;
        tokens.push(card.to_token_enum());
        return Ok(());
    }

    if verb == Token::PlayRoyal {
        let Some(card_token) = tokens.get(2) else {
            return Err(TokenError::InvalidFormat);
        };
        let card = card_token.card().ok_or(TokenError::UnknownCard)?;
        if !is_glasses_card(card) {
            return Ok(());
        }

        let snapshot_start = match card {
            Card::Standard {
                rank: Rank::Jack, ..
            }
            | Card::Joker(_) => 4,
            Card::Standard {
                rank: Rank::Eight | Rank::Queen | Rank::King,
                ..
            } => 3,
            _ => return Ok(()),
        };

        if tokens.len() == snapshot_start {
            append_glasses_snapshot_token_strings(tokens, state)?;
        }
    }

    Ok(())
}

fn parse_action_with_state(
    parts: &[Token],
    cursor: usize,
    state: &CutthroatState,
) -> Result<(Action, usize), TokenError> {
    let verb = parts
        .get(cursor)
        .copied()
        .ok_or(TokenError::InvalidFormat)?;
    let mut next = cursor + 1;
    match verb {
        Token::Draw => {
            let expected = state.deck.first().copied();
            match parts.get(next).copied() {
                Some(tok) => {
                    let card = tok.card().ok_or(TokenError::UnknownCard)?;
                    if Some(card) != expected {
                        return Err(TokenError::InvalidFormat);
                    }
                    next += 1;
                }
                None => return Err(TokenError::InvalidFormat),
            }
            Ok((Action::Draw, next))
        }
        Token::Pass => Ok((Action::Pass, next)),
        Token::Counter => {
            let card = parse_card_at(parts, next)?;
            next += 1;
            Ok((Action::CounterTwo { two_card: card }, next))
        }
        Token::Resolve => {
            let action = match &state.phase {
                Phase::Countering(_) => Action::CounterPass,
                Phase::ResolvingThree { .. } => {
                    let card = parse_card_at(parts, next)?;
                    next += 1;
                    Action::ResolveThreePick {
                        card_from_scrap: card,
                    }
                }
                Phase::ResolvingFour { .. } => {
                    if parts.get(next).copied() != Some(Token::Discard) {
                        return Err(TokenError::InvalidFormat);
                    }
                    let card = parse_card_at(parts, next + 1)?;
                    next += 2;
                    Action::ResolveFourDiscard { card }
                }
                _ => return Err(TokenError::UnknownAction),
            };
            Ok((action, next))
        }
        Token::Discard => {
            let card = parse_card_at(parts, next)?;
            next += 1;
            let action = match &state.phase {
                Phase::ResolvingFive { .. } => Action::ResolveFiveDiscard { card },
                Phase::ResolvingSeven { .. } => {
                    resolve_seven_choose_action(state, card, SevenPlay::Discard)?
                }
                _ => return Err(TokenError::UnknownAction),
            };
            Ok((action, next))
        }
        Token::Points => {
            let card = parse_card_at(parts, next)?;
            next += 1;
            let action = match &state.phase {
                Phase::ResolvingSeven { .. } => {
                    resolve_seven_choose_action(state, card, SevenPlay::Points)?
                }
                _ => Action::PlayPoints { card },
            };
            Ok((action, next))
        }
        Token::Scuttle => {
            let card = parse_card_at(parts, next)?;
            let target = parse_card_at(parts, next + 1)?;
            next += 2;
            let action = match &state.phase {
                Phase::ResolvingSeven { .. } => {
                    resolve_seven_choose_action(state, card, SevenPlay::Scuttle { target })?
                }
                _ => Action::Scuttle {
                    card,
                    target_point_base: target,
                },
            };
            Ok((action, next))
        }
        Token::PlayRoyal => {
            let card = parse_card_at(parts, next)?;
            next += 1;
            let mut target = None;

            match card {
                Card::Standard {
                    rank: Rank::Jack, ..
                }
                | Card::Joker(_) => {
                    target = Some(parse_card_at(parts, next)?);
                    next += 1;
                }
                Card::Standard {
                    rank: Rank::Eight | Rank::Queen | Rank::King,
                    ..
                } => {}
                _ => return Err(TokenError::UnknownAction),
            }

            if is_glasses_card(card) {
                next = parse_glasses_snapshot_tokens(parts, next, state)?;
            }

            let action = match &state.phase {
                Phase::ResolvingSeven { .. } => match card {
                    Card::Standard {
                        rank: Rank::Jack, ..
                    } => resolve_seven_choose_action(
                        state,
                        card,
                        SevenPlay::Jack {
                            target: target.ok_or(TokenError::InvalidFormat)?,
                        },
                    )?,
                    Card::Joker(_) => resolve_seven_choose_action(
                        state,
                        card,
                        SevenPlay::Joker {
                            target: target.ok_or(TokenError::InvalidFormat)?,
                        },
                    )?,
                    Card::Standard {
                        rank: Rank::Eight | Rank::Queen | Rank::King,
                        ..
                    } => {
                        if target.is_some() {
                            return Err(TokenError::InvalidFormat);
                        }
                        resolve_seven_choose_action(state, card, SevenPlay::Royal)?
                    }
                    _ => return Err(TokenError::UnknownAction),
                },
                _ => match card {
                    Card::Standard {
                        rank: Rank::Jack, ..
                    } => Action::PlayJack {
                        jack: card,
                        target_point_base: target.ok_or(TokenError::InvalidFormat)?,
                    },
                    Card::Joker(_) => Action::PlayJoker {
                        joker: card,
                        target_royal_card: target.ok_or(TokenError::InvalidFormat)?,
                    },
                    Card::Standard {
                        rank: Rank::Eight | Rank::Queen | Rank::King,
                        ..
                    } => {
                        if target.is_some() {
                            return Err(TokenError::InvalidFormat);
                        }
                        Action::PlayRoyal { card }
                    }
                    _ => return Err(TokenError::UnknownAction),
                },
            };

            Ok((action, next))
        }
        Token::OneOff => {
            let card = parse_card_at(parts, next)?;
            next += 1;
            let (target, after_target) = parse_inline_oneoff_target(parts, next, card, state)?;
            next = after_target;
            let action = match &state.phase {
                Phase::ResolvingSeven { .. } => {
                    resolve_seven_choose_action(state, card, SevenPlay::OneOff { target })?
                }
                _ => Action::PlayOneOff { card, target },
            };
            Ok((action, next))
        }
        _ => Err(TokenError::UnknownAction),
    }
}

fn encode_inline_oneoff_target(target: &OneOffTarget, tokens: &mut Vec<Token>) {
    match target {
        OneOffTarget::None => {}
        OneOffTarget::Player { seat } => {
            if let Ok(tok) = seat_to_token(*seat) {
                tokens.push(tok);
            }
        }
        OneOffTarget::Point { base } => tokens.push(base.to_token_enum()),
        OneOffTarget::Royal { card } => tokens.push(card.to_token_enum()),
        OneOffTarget::Jack { card } => tokens.push(card.to_token_enum()),
        OneOffTarget::Joker { card } => tokens.push(card.to_token_enum()),
    }
}

fn parse_inline_oneoff_target(
    parts: &[Token],
    cursor: usize,
    oneoff_card: Card,
    state: &CutthroatState,
) -> Result<(OneOffTarget, usize), TokenError> {
    let Some(tok) = parts.get(cursor).copied() else {
        return Ok((OneOffTarget::None, cursor));
    };

    if let Ok(seat) = parse_seat_token(tok) {
        if matches!(
            oneoff_card,
            Card::Standard {
                rank: Rank::Four,
                ..
            }
        ) {
            return Ok((OneOffTarget::Player { seat }, cursor + 1));
        }
        return Ok((OneOffTarget::None, cursor));
    }

    if let Some(target_card) = tok.card() {
        let target = infer_inline_oneoff_card_target(state, target_card)?;
        return Ok((target, cursor + 1));
    }

    Ok((OneOffTarget::None, cursor))
}

fn infer_inline_oneoff_card_target(
    state: &CutthroatState,
    target_card: Card,
) -> Result<OneOffTarget, TokenError> {
    for player in &state.players {
        for stack in &player.points {
            if stack.base == target_card {
                return Ok(OneOffTarget::Point { base: target_card });
            }
        }
    }
    for player in &state.players {
        for stack in &player.royals {
            if stack.base == target_card {
                return Ok(OneOffTarget::Royal { card: target_card });
            }
        }
    }
    for player in &state.players {
        for stack in &player.points {
            if let Some(top) = stack.jacks.last()
                && top.card == target_card
            {
                return Ok(OneOffTarget::Jack { card: target_card });
            }
        }
    }
    for player in &state.players {
        for stack in &player.royals {
            if let Some(top) = stack.jokers.last()
                && top.card == target_card
            {
                return Ok(OneOffTarget::Joker { card: target_card });
            }
        }
    }
    Err(TokenError::InvalidFormat)
}

fn ensure_seven_revealed_contains_card(
    state: &CutthroatState,
    card: Card,
) -> Result<(), TokenError> {
    let Phase::ResolvingSeven { revealed, .. } = &state.phase else {
        return Err(TokenError::UnknownAction);
    };
    if revealed.iter().any(|revealed_card| *revealed_card == card) {
        Ok(())
    } else {
        Err(TokenError::InvalidFormat)
    }
}

fn resolve_seven_choose_action(
    state: &CutthroatState,
    card: Card,
    play: SevenPlay,
) -> Result<Action, TokenError> {
    ensure_seven_revealed_contains_card(state, card)?;
    Ok(Action::ResolveSevenChoose { card, play })
}

fn is_glasses_card(card: Card) -> bool {
    matches!(
        card,
        Card::Standard {
            rank: Rank::Eight,
            ..
        }
    )
}

fn parse_glasses_snapshot_tokens(
    parts: &[Token],
    mut cursor: usize,
    state: &CutthroatState,
) -> Result<usize, TokenError> {
    let [first_opp, second_opp] = state.opponent_seat_order_for_current_actor();
    let first_tok = parts
        .get(cursor)
        .copied()
        .ok_or(TokenError::InvalidFormat)?;
    if parse_seat_token(first_tok).ok() != Some(first_opp) {
        return Err(TokenError::InvalidFormat);
    }

    for opp in [first_opp, second_opp] {
        let seat_tok = parts
            .get(cursor)
            .copied()
            .ok_or(TokenError::InvalidFormat)?;
        if parse_seat_token(seat_tok)? != opp {
            return Err(TokenError::InvalidFormat);
        }
        cursor += 1;
        for expected in &state.players[opp as usize].hand {
            let card = parse_card_at(parts, cursor)?;
            if card != *expected {
                return Err(TokenError::InvalidFormat);
            }
            cursor += 1;
        }
    }
    if cursor == parts.len() || is_action_seat_then_verb(parts, cursor) {
        Ok(cursor)
    } else {
        Err(TokenError::InvalidFormat)
    }
}

fn append_glasses_snapshot_tokens(
    tokens: &mut Vec<Token>,
    state_before: &CutthroatState,
    actor_seat: Seat,
) -> Result<(), TokenizeError> {
    for opp in opponent_seat_order(actor_seat) {
        tokens.push(seat_to_token(opp)?);
        for card in &state_before.players[opp as usize].hand {
            tokens.push(card.to_token_enum());
        }
    }
    Ok(())
}

fn append_glasses_snapshot_token_strings(
    tokens: &mut Vec<Token>,
    state: &CutthroatState,
) -> Result<(), TokenError> {
    for opp in state.opponent_seat_order_for_current_actor() {
        if opp >= PLAYER_COUNT {
            return Err(TokenError::InvalidFormat);
        }
        tokens.push(seat_to_token(opp).map_err(|_| TokenError::InvalidFormat)?);
        for card in &state.players[opp as usize].hand {
            tokens.push(card.to_token_enum());
        }
    }
    Ok(())
}

fn opponent_seat_order(actor_seat: Seat) -> [Seat; 2] {
    [
        (actor_seat + 1) % PLAYER_COUNT,
        (actor_seat + 2) % PLAYER_COUNT,
    ]
}

fn parse_card_at(parts: &[Token], cursor: usize) -> Result<Card, TokenError> {
    let tok = parts
        .get(cursor)
        .copied()
        .ok_or(TokenError::InvalidFormat)?;
    tok.card().ok_or(TokenError::UnknownCard)
}

fn is_action_seat_then_verb(parts: &[Token], index: usize) -> bool {
    let Some(seat_tok) = parts.get(index) else {
        return false;
    };
    if parse_seat_token(*seat_tok).is_err() {
        return false;
    }
    let Some(verb) = parts.get(index + 1) else {
        return false;
    };
    verb.is_verb()
}

fn parse_seat_token(token: Token) -> Result<Seat, TokenError> {
    let Some(num) = token.seat() else {
        return Err(TokenError::InvalidFormat);
    };
    if num >= PLAYER_COUNT {
        return Err(TokenError::InvalidFormat);
    }
    Ok(num)
}

fn seat_to_token(seat: Seat) -> Result<Token, TokenizeError> {
    Token::from_seat(seat).ok_or(TokenizeError::InvalidSeat)
}

#[cfg(test)]
mod tests {
    use super::{
        TokenError, TokenLog, append_action, encode_header, parse, parse_action_tokens_for_state,
        replay,
    };
    use crate::action::Action;
    use crate::state::{CutthroatState, Phase};

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

    #[test]
    fn parse_rejects_bare_card_action_token() {
        let state = CutthroatState::new_with_deck(0, crate::card::full_deck_with_jokers());
        let err = parse_action_tokens_for_state("P0 AC", &state)
            .expect_err("bare-card action token must be rejected");
        assert!(matches!(err, TokenError::UnknownAction));
    }

    #[test]
    fn parse_resolving_four_requires_resolve_discard_shape() {
        let mut state = CutthroatState::new_with_deck(0, crate::card::full_deck_with_jokers());
        state.phase = Phase::ResolvingFour {
            seat: 0,
            base_player: 1,
            remaining: 2,
        };

        let parsed = parse_action_tokens_for_state("P0 resolve discard AC", &state)
            .expect("resolve-four token shape should parse");
        assert!(matches!(parsed.1, Action::ResolveFourDiscard { .. }));

        let err = parse_action_tokens_for_state("P0 discard AC", &state)
            .expect_err("discard-only should be rejected in resolving four");
        assert!(matches!(err, TokenError::UnknownAction));
    }

    #[test]
    fn parse_resolving_five_requires_discard_shape() {
        let mut state = CutthroatState::new_with_deck(0, crate::card::full_deck_with_jokers());
        state.phase = Phase::ResolvingFive {
            seat: 0,
            base_player: 0,
            discarded: false,
        };

        let parsed = parse_action_tokens_for_state("P0 discard AC", &state)
            .expect("resolve-five discard token should parse");
        assert!(matches!(parsed.1, Action::ResolveFiveDiscard { .. }));

        let err = parse_action_tokens_for_state("P0 resolve discard AC", &state)
            .expect_err("resolve-discard should be rejected in resolving five");
        assert!(matches!(err, TokenError::UnknownAction));
    }

    #[test]
    fn append_draw_includes_drawn_card_token() {
        let state = CutthroatState::new_with_deck(0, crate::card::full_deck_with_jokers());
        let mut tokens = encode_header(state.dealer, &state.deck);
        append_action(&mut tokens, &state, state.turn, &Action::Draw).expect("draw encodes");
        let draw_fragment = format!(
            "P{} draw {}",
            state.turn,
            state.deck.first().expect("deck has top card").to_token()
        );
        assert!(
            tokens.contains(&draw_fragment),
            "expected draw fragment `{draw_fragment}` in `{tokens}`"
        );
    }

    #[test]
    fn parse_rejects_draw_without_card_in_full_tokenlog() {
        let deck = crate::card::full_deck_with_jokers();
        let tokenlog = format!("{} P0 draw", encode_header(0, &deck));
        let err = parse(&tokenlog).expect_err("full tokenlog draw must include card");
        assert!(matches!(
            err,
            TokenError::UnknownCard | TokenError::InvalidFormat
        ));
    }

    #[test]
    fn parse_action_tokens_allows_draw_without_card() {
        let state = CutthroatState::new_with_deck(0, crate::card::full_deck_with_jokers());
        let parsed = parse_action_tokens_for_state("P0 draw", &state)
            .expect("action token draw without card should parse");
        assert_eq!(parsed, (0, Action::Draw));
    }
}
