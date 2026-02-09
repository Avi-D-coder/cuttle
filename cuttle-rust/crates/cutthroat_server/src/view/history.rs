use crate::game_runtime::{GameEntry, SeatEntry};
use cutthroat_engine::state::{PhaseView, PublicCard};
use cutthroat_engine::{
    Action, Card, CutthroatState, OneOffTarget, PublicView, Seat, SevenPlay, parse_tokenlog,
};
use std::collections::{HashMap, HashSet};

const LOG_TAIL_LIMIT: usize = 60;

pub(crate) fn build_history_log_for_viewer(game: &GameEntry, viewer: Seat) -> Vec<String> {
    build_history_log_for_viewer_with_limit(game, viewer, None)
}

pub(crate) fn build_history_log_for_viewer_with_limit(
    game: &GameEntry,
    viewer: Seat,
    max_actions: Option<usize>,
) -> Vec<String> {
    let Ok(parsed) = parse_tokenlog(&game.tokenlog_full) else {
        return Vec::new();
    };
    let mut state = CutthroatState::new_with_deck(parsed.dealer, parsed.deck);
    let seat_names = seat_name_map(&game.seats);
    let mut lines = Vec::new();
    let action_limit = max_actions.unwrap_or(usize::MAX);

    for (idx, (actor_seat, action)) in parsed.actions.into_iter().enumerate() {
        if idx >= action_limit {
            break;
        }
        if state.apply(actor_seat, action.clone()).is_err() {
            break;
        }
        let view = state.public_view(viewer);
        let visible_tokens = collect_visible_tokens(&view);
        lines.push(format_history_line(
            &action,
            actor_seat,
            &seat_names,
            &visible_tokens,
        ));
    }

    if lines.len() > LOG_TAIL_LIMIT {
        lines.drain(0..(lines.len() - LOG_TAIL_LIMIT));
    }
    lines
}

fn seat_name_map(seats: &[SeatEntry]) -> HashMap<Seat, String> {
    seats
        .iter()
        .map(|seat| (seat.seat, seat.username.clone()))
        .collect()
}

fn seat_name(seat: Seat, seat_names: &HashMap<Seat, String>) -> String {
    seat_names
        .get(&seat)
        .cloned()
        .unwrap_or_else(|| format!("Player {}", seat + 1))
}

fn collect_visible_tokens(view: &PublicView) -> HashSet<String> {
    let mut visible = HashSet::new();

    for token in &view.scrap {
        visible.insert(token.clone());
    }

    for player in &view.players {
        for hand_card in &player.hand {
            if let PublicCard::Known(token) = hand_card {
                visible.insert(token.clone());
            }
        }
        for point in &player.points {
            visible.insert(point.base.clone());
            for jack in &point.jacks {
                visible.insert(jack.clone());
            }
        }
        for royal in &player.royals {
            visible.insert(royal.base.clone());
            for joker in &royal.jokers {
                visible.insert(joker.clone());
            }
        }
        for frozen in &player.frozen {
            visible.insert(frozen.clone());
        }
    }

    match &view.phase {
        PhaseView::Countering { oneoff, twos, .. } => {
            add_action_tokens(oneoff, &mut visible);
            for two in twos {
                visible.insert(two.card.clone());
            }
        }
        PhaseView::ResolvingSeven { revealed_cards, .. } => {
            for token in revealed_cards {
                visible.insert(token.clone());
            }
        }
        _ => {}
    }

    visible
}

fn add_action_tokens(action: &Action, visible: &mut HashSet<String>) {
    match action {
        Action::PlayPoints { card } => {
            visible.insert(card.to_token());
        }
        Action::Scuttle {
            card,
            target_point_base,
        } => {
            visible.insert(card.to_token());
            visible.insert(target_point_base.to_token());
        }
        Action::PlayRoyal { card } => {
            visible.insert(card.to_token());
        }
        Action::PlayJack {
            jack,
            target_point_base,
        } => {
            visible.insert(jack.to_token());
            visible.insert(target_point_base.to_token());
        }
        Action::PlayJoker {
            joker,
            target_royal_card,
        } => {
            visible.insert(joker.to_token());
            visible.insert(target_royal_card.to_token());
        }
        Action::PlayOneOff { card, target } => {
            visible.insert(card.to_token());
            add_oneoff_target_tokens(target, visible);
        }
        Action::CounterTwo { two_card } => {
            visible.insert(two_card.to_token());
        }
        Action::ResolveThreePick { card_from_scrap } => {
            visible.insert(card_from_scrap.to_token());
        }
        Action::ResolveFourDiscard { card } => {
            visible.insert(card.to_token());
        }
        Action::ResolveFiveDiscard { card } => {
            visible.insert(card.to_token());
        }
        Action::ResolveSevenChoose { play, .. } => add_seven_play_tokens(play, visible),
        Action::Draw | Action::Pass | Action::CounterPass => {}
    }
}

fn add_oneoff_target_tokens(target: &OneOffTarget, visible: &mut HashSet<String>) {
    match target {
        OneOffTarget::Point { base } => {
            visible.insert(base.to_token());
        }
        OneOffTarget::Royal { card } => {
            visible.insert(card.to_token());
        }
        OneOffTarget::Jack { card } => {
            visible.insert(card.to_token());
        }
        OneOffTarget::Joker { card } => {
            visible.insert(card.to_token());
        }
        OneOffTarget::None | OneOffTarget::Player { .. } => {}
    }
}

fn add_seven_play_tokens(play: &SevenPlay, visible: &mut HashSet<String>) {
    match play {
        SevenPlay::Scuttle { target } => {
            visible.insert(target.to_token());
        }
        SevenPlay::Jack { target } => {
            visible.insert(target.to_token());
        }
        SevenPlay::Joker { target } => {
            visible.insert(target.to_token());
        }
        SevenPlay::OneOff { target } => {
            add_oneoff_target_tokens(target, visible);
        }
        SevenPlay::Points | SevenPlay::Royal | SevenPlay::Discard => {}
    }
}

fn format_history_line(
    action: &Action,
    actor_seat: Seat,
    seat_names: &HashMap<Seat, String>,
    visible_tokens: &HashSet<String>,
) -> String {
    let actor = seat_name(actor_seat, seat_names);
    match action {
        Action::Draw => format!("{} drew a card.", actor),
        Action::Pass => format!("{} passed.", actor),
        Action::PlayPoints { card } => {
            format!(
                "{} played the {} for points.",
                actor,
                card_name_for_history(*card, visible_tokens)
            )
        }
        Action::Scuttle {
            card,
            target_point_base,
        } => format!(
            "{} scuttled the {} with the {}.",
            actor,
            card_name_for_history(*target_point_base, visible_tokens),
            card_name_for_history(*card, visible_tokens)
        ),
        Action::PlayRoyal { card } => format!(
            "{} played the {} as a royal.",
            actor,
            card_name_for_history(*card, visible_tokens)
        ),
        Action::PlayJack {
            jack,
            target_point_base,
        } => format!(
            "{} stole the {} with the {}.",
            actor,
            card_name_for_history(*target_point_base, visible_tokens),
            card_name_for_history(*jack, visible_tokens)
        ),
        Action::PlayJoker {
            joker,
            target_royal_card,
        } => format!(
            "{} played the {} on the {}.",
            actor,
            card_name_for_history(*joker, visible_tokens),
            card_name_for_history(*target_royal_card, visible_tokens)
        ),
        Action::PlayOneOff { card, target } => format!(
            "{} played the {} as a one-off{}.",
            actor,
            card_name_for_history(*card, visible_tokens),
            oneoff_target_text(target, seat_names, visible_tokens)
        ),
        Action::CounterTwo { two_card } => format!(
            "{} played the {} to counter.",
            actor,
            card_name_for_history(*two_card, visible_tokens)
        ),
        Action::CounterPass => format!("{} passed counter.", actor),
        Action::ResolveThreePick { card_from_scrap } => format!(
            "{} took the {} from scrap.",
            actor,
            card_name_for_history(*card_from_scrap, visible_tokens)
        ),
        Action::ResolveFourDiscard { card } => format!(
            "{} discarded the {}.",
            actor,
            card_name_for_history(*card, visible_tokens)
        ),
        Action::ResolveFiveDiscard { card } => format!(
            "{} discarded the {}.",
            actor,
            card_name_for_history(*card, visible_tokens)
        ),
        Action::ResolveSevenChoose { source_index, play } => format!(
            "{} resolved seven from revealed card {}{}.",
            actor,
            source_index + 1,
            seven_play_text(play, seat_names, visible_tokens)
        ),
    }
}

fn oneoff_target_text(
    target: &OneOffTarget,
    seat_names: &HashMap<Seat, String>,
    visible_tokens: &HashSet<String>,
) -> String {
    match target {
        OneOffTarget::None => String::new(),
        OneOffTarget::Player { seat } => format!(", targeting {}", seat_name(*seat, seat_names)),
        OneOffTarget::Point { base } => format!(
            ", targeting the {}",
            card_name_for_history(*base, visible_tokens)
        ),
        OneOffTarget::Royal { card } => format!(
            ", targeting the {}",
            card_name_for_history(*card, visible_tokens)
        ),
        OneOffTarget::Jack { card } => format!(
            ", targeting the {}",
            card_name_for_history(*card, visible_tokens)
        ),
        OneOffTarget::Joker { card } => format!(
            ", targeting the {}",
            card_name_for_history(*card, visible_tokens)
        ),
    }
}

fn seven_play_text(
    play: &SevenPlay,
    seat_names: &HashMap<Seat, String>,
    visible_tokens: &HashSet<String>,
) -> String {
    match play {
        SevenPlay::Points => " as points".to_string(),
        SevenPlay::Scuttle { target } => format!(
            " as scuttle targeting {}",
            card_name_for_history(*target, visible_tokens)
        ),
        SevenPlay::Royal => " as a royal".to_string(),
        SevenPlay::Jack { target } => format!(
            " as a jack targeting {}",
            card_name_for_history(*target, visible_tokens)
        ),
        SevenPlay::Joker { target } => format!(
            " as a joker targeting {}",
            card_name_for_history(*target, visible_tokens)
        ),
        SevenPlay::OneOff { target } => {
            format!(
                " as a one-off{}",
                oneoff_target_text(target, seat_names, visible_tokens)
            )
        }
        SevenPlay::Discard => " as discard".to_string(),
    }
}

fn card_name_for_history(card: Card, visible_tokens: &HashSet<String>) -> String {
    let token = card.to_token();
    if !visible_tokens.contains(&token) {
        return "Unknown card".to_string();
    }
    card_token_to_human(&token)
}

fn card_token_to_human(token: &str) -> String {
    if token == "J0" {
        return "Joker 0".to_string();
    }
    if token == "J1" {
        return "Joker 1".to_string();
    }
    let mut chars = token.chars();
    let rank = chars.next().unwrap_or('?');
    let suit = match chars.next().unwrap_or('?') {
        'C' => '♣',
        'D' => '♦',
        'H' => '♥',
        'S' => '♠',
        _ => '?',
    };
    format!("{}{}", rank, suit)
}
