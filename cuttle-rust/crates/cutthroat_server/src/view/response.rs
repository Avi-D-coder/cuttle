use crate::game_runtime::{GameEntry, SeatEntry};
use cutthroat_engine::state::PublicCard;
use cutthroat_engine::{
    Action, CutthroatState, Phase, Seat, SeatView, Token, TokenLog, append_action,
    encode_action_token_vec_for_input, encode_header, join_tokens, parse_token_slice,
};

const UNKNOWN_CARD_TOKEN: &str = "UNKNOWN";

#[cfg(test)]
pub(crate) fn usernames_from_seats(seats: &[SeatEntry]) -> Option<[String; 3]> {
    let mut usernames: [Option<String>; 3] = [None, None, None];
    for seat in seats {
        let idx = seat.seat as usize;
        if idx < 3 {
            usernames[idx] = Some(seat.username.clone());
        }
    }
    Some([
        usernames[0].clone()?,
        usernames[1].clone()?,
        usernames[2].clone()?,
    ])
}

pub(crate) fn normal_lobby_name(seats: &[SeatEntry]) -> String {
    let mut by_seat = [
        String::from("Open"),
        String::from("Open"),
        String::from("Open"),
    ];
    for seat in seats {
        let idx = seat.seat as usize;
        if idx < 3 {
            by_seat[idx] = seat.username.clone();
        }
    }
    format!("{} VS {} VS {}", by_seat[0], by_seat[1], by_seat[2])
}

pub(crate) fn build_spectator_view(game: &GameEntry) -> SeatView {
    let viewer = match &game.engine.phase {
        Phase::ResolvingSeven { seat, .. } => *seat,
        _ => game.engine.turn,
    };
    let mut view = game.engine.public_view(viewer);
    for (idx, player) in game.engine.players.iter().enumerate() {
        if let Some(player_view) = view.players.get_mut(idx) {
            player_view.hand = player
                .hand
                .iter()
                .map(|card| PublicCard::Known(card.to_token_enum()))
                .collect();
            player_view.frozen = player
                .frozen
                .iter()
                .map(|card| card.card.to_token())
                .collect();
        }
    }
    view
}

pub(crate) fn format_action(action: &Action) -> String {
    match action {
        Action::Draw => "draw".to_string(),
        Action::Pass => "pass".to_string(),
        Action::PlayPoints { .. } => "points".to_string(),
        Action::Scuttle { .. } => "scuttle".to_string(),
        Action::PlayRoyal { .. } => "royal".to_string(),
        Action::PlayJack { .. } => "jack".to_string(),
        Action::PlayJoker { .. } => "joker".to_string(),
        Action::PlayOneOff { .. } => "oneoff".to_string(),
        Action::CounterTwo { .. } => "counter_two".to_string(),
        Action::CounterPass => "counter_pass".to_string(),
        Action::ResolveThreePick { .. } => "resolve_three".to_string(),
        Action::ResolveFourDiscard { .. } => "resolve_four".to_string(),
        Action::ResolveFiveDiscard { .. } => "resolve_five".to_string(),
        Action::ResolveSevenChoose { .. } => "resolve_seven".to_string(),
    }
}

pub(crate) fn legal_action_tokens_for_seat(state: &CutthroatState, seat: Seat) -> Vec<String> {
    let mut legal_actions = state.legal_actions(seat);
    legal_actions.sort_by_key(format_action);
    legal_actions
        .into_iter()
        .filter_map(|action| encode_action_token_vec_for_input(state, seat, &action).ok())
        .map(|tokens| join_tokens(&tokens))
        .collect()
}

pub(crate) fn serialize_tokenlog(transcript: &TokenLog) -> String {
    let mut encoded = encode_header(transcript.dealer, &transcript.deck);
    let mut state = CutthroatState::new_with_deck(transcript.dealer, transcript.deck.clone());
    for (seat, action) in &transcript.actions {
        if append_action(&mut encoded, &state, *seat, action).is_err() {
            break;
        }
        if state.apply(*seat, action.clone()).is_err() {
            break;
        }
    }
    encoded
}

pub(crate) fn redact_tokenlog_for_client(transcript: &TokenLog, viewer: Option<Seat>) -> String {
    let mut redacted = encode_header(transcript.dealer, &[]);
    let mut state = CutthroatState::new_with_deck(transcript.dealer, transcript.deck.clone());
    for (seat, action) in &transcript.actions {
        let should_hide_draw = matches!(action, Action::Draw) && viewer.is_some_and(|v| v != *seat);
        if should_hide_draw {
            if !redacted.is_empty() {
                redacted.push(' ');
            }
            redacted.push_str(&format!("P{} draw {}", seat, UNKNOWN_CARD_TOKEN));
        } else if append_action(&mut redacted, &state, *seat, action).is_err() {
            break;
        }
        if state.apply(*seat, action.clone()).is_err() {
            break;
        }
    }
    redacted
}

pub(crate) fn redact_tokenlog_tokens_for_client(
    transcript: &TokenLog,
    viewer: Option<Seat>,
) -> Vec<Token> {
    parse_token_slice(&redact_tokenlog_for_client(transcript, viewer)).unwrap_or_default()
}
