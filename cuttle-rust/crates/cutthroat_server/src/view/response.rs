use crate::game_runtime::{GameEntry, SeatEntry};
use cutthroat_engine::state::PublicCard;
use cutthroat_engine::{
    Action, CutthroatState, LastEventView, OneOffTarget, Phase, PublicView, Seat, SevenPlay,
    TokenLog, append_action, encode_action_token_vec_for_input, encode_header, join_tokens,
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

pub(crate) fn build_spectator_view(game: &GameEntry) -> PublicView {
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
                .map(|card| PublicCard::Known(card.to_token()))
                .collect();
            player_view.frozen = player
                .frozen
                .iter()
                .map(|card| card.card.to_token())
                .collect();
        }
    }
    view.deck_count = 0;
    view.last_event = game.last_event.clone();
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

fn oneoff_target_fields(target: &OneOffTarget) -> (Option<String>, Option<Seat>, Option<String>) {
    match target {
        OneOffTarget::None => (None, None, None),
        OneOffTarget::Player { seat } => (None, Some(*seat), Some("player".to_string())),
        OneOffTarget::Point { base } => (Some(base.to_token()), None, Some("point".to_string())),
        OneOffTarget::Royal { card } => (Some(card.to_token()), None, Some("royal".to_string())),
        OneOffTarget::Jack { card } => (Some(card.to_token()), None, Some("jack".to_string())),
        OneOffTarget::Joker { card } => (Some(card.to_token()), None, Some("joker".to_string())),
    }
}

pub(crate) fn build_last_event(
    actor: Seat,
    action: &Action,
    phase_before: &Phase,
) -> LastEventView {
    let action_kind = format_action(action);
    let mut change = "main".to_string();
    let mut source_token: Option<String> = None;
    let source_zone: Option<String>;
    let mut target_token: Option<String> = None;
    let mut target_seat: Option<Seat> = None;
    let mut target_type: Option<String> = None;
    let mut oneoff_rank: Option<u8> = None;

    match action {
        Action::Draw => {
            source_zone = Some("deck".to_string());
        }
        Action::Pass => {
            source_zone = Some("deck".to_string());
        }
        Action::PlayPoints { card } => {
            source_zone = Some("hand".to_string());
            source_token = Some(card.to_token());
        }
        Action::Scuttle {
            card,
            target_point_base,
        } => {
            change = "scuttle".to_string();
            source_zone = Some("hand".to_string());
            source_token = Some(card.to_token());
            target_token = Some(target_point_base.to_token());
            target_type = Some("point".to_string());
        }
        Action::PlayRoyal { card } => {
            source_zone = Some("hand".to_string());
            source_token = Some(card.to_token());
        }
        Action::PlayJack {
            jack,
            target_point_base,
        } => {
            change = "jack".to_string();
            source_zone = Some("hand".to_string());
            source_token = Some(jack.to_token());
            target_token = Some(target_point_base.to_token());
            target_type = Some("point".to_string());
        }
        Action::PlayJoker {
            joker,
            target_royal_card,
        } => {
            change = "joker".to_string();
            source_zone = Some("hand".to_string());
            source_token = Some(joker.to_token());
            target_token = Some(target_royal_card.to_token());
            target_type = Some("royal".to_string());
        }
        Action::PlayOneOff { card, target } => {
            change = "counter".to_string();
            source_zone = Some("hand".to_string());
            source_token = Some(card.to_token());
            oneoff_rank = card.rank_value();
            let (target_token_val, target_seat_val, target_type_val) = oneoff_target_fields(target);
            target_token = target_token_val;
            target_seat = target_seat_val;
            target_type = target_type_val;
        }
        Action::CounterTwo { two_card } => {
            change = "counter".to_string();
            source_zone = Some("hand".to_string());
            source_token = Some(two_card.to_token());
        }
        Action::CounterPass => {
            change = "counter".to_string();
            source_zone = Some("counter".to_string());
            source_token = Some("pass".to_string());
        }
        Action::ResolveThreePick { card_from_scrap } => {
            change = "resolve".to_string();
            source_zone = Some("scrap".to_string());
            source_token = Some(card_from_scrap.to_token());
        }
        Action::ResolveFourDiscard { card } => {
            change = "resolve".to_string();
            source_zone = Some("hand".to_string());
            source_token = Some(card.to_token());
        }
        Action::ResolveFiveDiscard { card } => {
            change = "resolve".to_string();
            source_zone = Some("hand".to_string());
            source_token = Some(card.to_token());
        }
        Action::ResolveSevenChoose { card, play } => {
            source_zone = Some("reveal".to_string());
            source_token = Some(card.to_token());
            match play {
                SevenPlay::Points => {
                    change = "main".to_string();
                }
                SevenPlay::Scuttle { target } => {
                    change = "scuttle".to_string();
                    target_token = Some(target.to_token());
                    target_type = Some("point".to_string());
                }
                SevenPlay::Royal => {
                    change = "main".to_string();
                }
                SevenPlay::Jack { target } => {
                    change = "sevenJack".to_string();
                    target_token = Some(target.to_token());
                    target_type = Some("point".to_string());
                }
                SevenPlay::Joker { target } => {
                    change = "joker".to_string();
                    target_token = Some(target.to_token());
                    target_type = Some("royal".to_string());
                }
                SevenPlay::OneOff { target } => {
                    change = "resolve".to_string();
                    let (target_token_val, target_seat_val, target_type_val) =
                        oneoff_target_fields(target);
                    target_token = target_token_val;
                    target_seat = target_seat_val;
                    target_type = target_type_val;
                }
                SevenPlay::Discard => {
                    change = "resolve".to_string();
                }
            }
        }
    }

    if let Phase::Countering(counter) = phase_before
        && matches!(action, Action::CounterPass | Action::CounterTwo { .. })
    {
        if let Action::PlayOneOff { target, card } = &counter.oneoff {
            oneoff_rank = oneoff_rank.or(card.rank_value());
            if target_type.is_none() {
                let (target_token_val, target_seat_val, target_type_val) =
                    oneoff_target_fields(target);
                target_token = target_token.or(target_token_val);
                target_seat = target_seat.or(target_seat_val);
                target_type = target_type.or(target_type_val);
            }
        }

        if matches!(action, Action::CounterPass) {
            let next_after_pass = (counter.next_seat + 1) % 3;
            if next_after_pass == counter.rotation_anchor {
                change = "resolve".to_string();
            }
        }
    }

    LastEventView {
        actor,
        action_kind,
        change,
        source_token,
        source_zone,
        target_token,
        target_seat,
        target_type,
        oneoff_rank,
    }
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
