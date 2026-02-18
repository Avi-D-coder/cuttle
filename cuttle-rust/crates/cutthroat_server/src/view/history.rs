use crate::game_runtime::{GameEntry, SeatEntry};
use cutthroat_engine::state::{PhaseView, PublicCard};
use cutthroat_engine::{
    Action, Card, CutthroatState, OneOffTarget, Phase, Seat, SeatView, SevenPlay, Token,
};
use std::collections::{HashMap, HashSet};

const LOG_TAIL_LIMIT: usize = 60;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum HistoryAudience {
    Seat(Seat),
    Spectator,
}

pub(crate) fn build_history_log_for_audience(
    game: &GameEntry,
    audience: HistoryAudience,
) -> Vec<String> {
    build_history_log_for_audience_with_limit(game, audience, None)
}

pub(crate) fn build_history_log_for_audience_with_limit(
    game: &GameEntry,
    audience: HistoryAudience,
    max_actions: Option<usize>,
) -> Vec<String> {
    let mut state =
        CutthroatState::new_with_deck(game.transcript.dealer, game.transcript.deck.clone());
    let seat_names = seat_name_map(&game.seats);
    let mut lines = Vec::new();
    let action_limit = max_actions.unwrap_or(usize::MAX);

    for (idx, (actor_seat, action)) in game.transcript.actions.iter().enumerate() {
        if idx >= action_limit {
            break;
        }
        let pre_view = view_for_audience(&state, audience);
        let revealed_cards = match &pre_view.phase {
            PhaseView::ResolvingSeven { revealed_cards, .. } => revealed_cards.clone(),
            _ => Vec::new(),
        };
        let mut visible_tokens = collect_visible_tokens(&pre_view);
        if state.apply(*actor_seat, action.clone()).is_err() {
            break;
        }
        let post_view = view_for_audience(&state, audience);
        visible_tokens.extend(collect_visible_tokens(&post_view));
        lines.push(format_history_line(
            action,
            *actor_seat,
            &seat_names,
            &visible_tokens,
            &revealed_cards,
        ));
    }

    if lines.len() > LOG_TAIL_LIMIT {
        lines.drain(0..(lines.len() - LOG_TAIL_LIMIT));
    }
    lines
}

pub(crate) fn build_history_log_for_viewer(game: &GameEntry, viewer: Seat) -> Vec<String> {
    build_history_log_for_audience(game, HistoryAudience::Seat(viewer))
}

fn view_for_audience(state: &CutthroatState, audience: HistoryAudience) -> SeatView {
    match audience {
        HistoryAudience::Seat(viewer) => state.public_view(viewer),
        HistoryAudience::Spectator => build_spectator_view(state),
    }
}

fn build_spectator_view(state: &CutthroatState) -> SeatView {
    let viewer = match &state.phase {
        Phase::ResolvingSeven { seat, .. } => *seat,
        _ => state.turn,
    };
    let mut view = state.public_view(viewer);
    for (idx, player) in state.players.iter().enumerate() {
        if let Some(player_view) = view.players.get_mut(idx) {
            player_view.hand = player
                .hand
                .iter()
                .map(|card| PublicCard::Known(card.to_token_enum()))
                .collect();
            player_view.frozen = player
                .frozen
                .iter()
                .map(|card| card.card.to_token_enum())
                .collect();
        }
    }
    view
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

fn collect_visible_tokens(view: &SeatView) -> HashSet<Token> {
    let mut visible = HashSet::new();

    for token in &view.scrap {
        visible.insert(*token);
    }

    for player in &view.players {
        for hand_card in &player.hand {
            if let PublicCard::Known(token) = hand_card {
                visible.insert(*token);
            }
        }
        for point in &player.points {
            visible.insert(point.base);
            for jack in &point.jacks {
                visible.insert(*jack);
            }
        }
        for royal in &player.royals {
            visible.insert(royal.base);
            for joker in &royal.jokers {
                visible.insert(*joker);
            }
        }
        for frozen in &player.frozen {
            visible.insert(*frozen);
        }
    }

    match &view.phase {
        PhaseView::Countering { oneoff, twos, .. } => {
            add_action_tokens(oneoff, &mut visible);
            for two in twos {
                visible.insert(two.card);
            }
        }
        PhaseView::ResolvingSeven { revealed_cards, .. } => {
            for token in revealed_cards {
                visible.insert(*token);
            }
        }
        _ => {}
    }

    visible
}

fn add_action_tokens(action: &Action, visible: &mut HashSet<Token>) {
    match action {
        Action::PlayPoints { card } => {
            visible.insert(card.to_token_enum());
        }
        Action::Scuttle {
            card,
            target_point_base,
        } => {
            visible.insert(card.to_token_enum());
            visible.insert(target_point_base.to_token_enum());
        }
        Action::PlayRoyal { card } => {
            visible.insert(card.to_token_enum());
        }
        Action::PlayJack {
            jack,
            target_point_base,
        } => {
            visible.insert(jack.to_token_enum());
            visible.insert(target_point_base.to_token_enum());
        }
        Action::PlayJoker {
            joker,
            target_royal_card,
        } => {
            visible.insert(joker.to_token_enum());
            visible.insert(target_royal_card.to_token_enum());
        }
        Action::PlayOneOff { card, target } => {
            visible.insert(card.to_token_enum());
            add_oneoff_target_tokens(target, visible);
        }
        Action::CounterTwo { two_card } => {
            visible.insert(two_card.to_token_enum());
        }
        Action::ResolveThreePick { card_from_scrap } => {
            visible.insert(card_from_scrap.to_token_enum());
        }
        Action::ResolveFourDiscard { card } => {
            visible.insert(card.to_token_enum());
        }
        Action::ResolveFiveDiscard { card } => {
            visible.insert(card.to_token_enum());
        }
        Action::ResolveSevenChoose { play, .. } => add_seven_play_tokens(play, visible),
        Action::Draw | Action::Pass | Action::CounterPass => {}
    }
}

fn add_oneoff_target_tokens(target: &OneOffTarget, visible: &mut HashSet<Token>) {
    match target {
        OneOffTarget::Point { base } => {
            visible.insert(base.to_token_enum());
        }
        OneOffTarget::Royal { card } => {
            visible.insert(card.to_token_enum());
        }
        OneOffTarget::Jack { card } => {
            visible.insert(card.to_token_enum());
        }
        OneOffTarget::Joker { card } => {
            visible.insert(card.to_token_enum());
        }
        OneOffTarget::None | OneOffTarget::Player { .. } => {}
    }
}

fn add_seven_play_tokens(play: &SevenPlay, visible: &mut HashSet<Token>) {
    match play {
        SevenPlay::Scuttle { target } => {
            visible.insert(target.to_token_enum());
        }
        SevenPlay::Jack { target } => {
            visible.insert(target.to_token_enum());
        }
        SevenPlay::Joker { target } => {
            visible.insert(target.to_token_enum());
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
    visible_tokens: &HashSet<Token>,
    revealed_cards: &[Token],
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
        Action::ResolveSevenChoose { card, play } => format!(
            "{} resolved seven with revealed {}{}{}.",
            actor,
            card_name_for_history(*card, visible_tokens),
            seven_reveal_text(revealed_cards, visible_tokens),
            seven_play_text(play, seat_names, visible_tokens)
        ),
    }
}

fn seven_reveal_text(revealed_cards: &[Token], visible_tokens: &HashSet<Token>) -> String {
    if revealed_cards.is_empty() {
        return String::new();
    }
    let names: Vec<String> = revealed_cards
        .iter()
        .take(2)
        .map(|token| {
            if visible_tokens.contains(token) {
                card_token_to_human(token.as_str())
            } else {
                "Unknown card".to_string()
            }
        })
        .collect();
    match names.as_slice() {
        [single] => format!(" (top reveal was {})", single),
        [first, second] => format!(" (top two were {} and {})", first, second),
        _ => String::new(),
    }
}

fn oneoff_target_text(
    target: &OneOffTarget,
    seat_names: &HashMap<Seat, String>,
    visible_tokens: &HashSet<Token>,
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
    visible_tokens: &HashSet<Token>,
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

fn card_name_for_history(card: Card, visible_tokens: &HashSet<Token>) -> String {
    let token = card.to_token_enum();
    if !visible_tokens.contains(&token) {
        return "Unknown card".to_string();
    }
    card_token_to_human(token.as_str())
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

#[cfg(test)]
mod tests {
    use super::format_history_line;
    use cutthroat_engine::{Action, Card, SevenPlay, Token};
    use std::collections::HashMap;
    use std::collections::HashSet;

    fn c(token: &str) -> Card {
        Card::from_token(token).expect("valid card token")
    }

    #[test]
    fn resolve_seven_history_line_includes_top_two_cards() {
        let mut seat_names = HashMap::new();
        seat_names.insert(0, "Avi3".to_string());
        seat_names.insert(1, "bbjme_test".to_string());
        seat_names.insert(2, "Spud_Spudoni".to_string());

        let mut visible_tokens = HashSet::new();
        visible_tokens.insert("6S".parse().expect("valid token"));
        visible_tokens.insert("KD".parse().expect("valid token"));

        let line = format_history_line(
            &Action::ResolveSevenChoose {
                card: c("6S"),
                play: SevenPlay::Points,
            },
            0,
            &seat_names,
            &visible_tokens,
            &[Token::P6S, Token::KD],
        );

        assert_eq!(
            line,
            "Avi3 resolved seven with revealed 6♠ (top two were 6♠ and K♦) as points."
        );
    }
}
