#![no_main]

use arbitrary::Arbitrary;
use cutthroat_engine::{
    Action, Card, CutthroatState, OneOffTarget, Phase, RuleError, SevenPlay, Winner,
    append_action, encode_header, full_deck_with_jokers, join_tokens, parse_token_slice, parse_tokenlog,
    replay_tokenlog,
};
use libfuzzer_sys::fuzz_target;

#[derive(Arbitrary, Debug)]
struct FuzzInput {
    dealer_seed: u8,
    deck_bytes: Vec<u8>,
    move_bytes: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct StateDigest {
    dealer: u8,
    turn: u8,
    phase: PhaseDigest,
    deck: Vec<Card>,
    scrap: Vec<Card>,
    players: Vec<PlayerDigest>,
    pass_streak_start: Option<u8>,
    pass_streak_len: u8,
    winner: Option<Winner>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum PhaseDigest {
    Main,
    Countering {
        base_player: u8,
        oneoff: Action,
        twos: Vec<(u8, Card)>,
        next_seat: u8,
        rotation_anchor: u8,
    },
    ResolvingThree {
        seat: u8,
        base_player: u8,
    },
    ResolvingFour {
        seat: u8,
        base_player: u8,
        remaining: u8,
    },
    ResolvingFive {
        seat: u8,
        base_player: u8,
        discarded: bool,
    },
    ResolvingSeven {
        seat: u8,
        base_player: u8,
        revealed: Vec<Card>,
    },
    GameOver,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct PlayerDigest {
    hand: Vec<Card>,
    points: Vec<PointStackDigest>,
    royals: Vec<RoyalStackDigest>,
    frozen: Vec<(Card, u8)>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct PointStackDigest {
    base: Card,
    base_owner: u8,
    jacks: Vec<(Card, u8)>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RoyalStackDigest {
    base: Card,
    base_owner: u8,
    jokers: Vec<(Card, u8)>,
}

fuzz_target!(|input: FuzzInput| {
    run_case(input);
});

fn run_case(input: FuzzInput) {
    let dealer = input.dealer_seed % 3;
    let deck = shuffled_deck(&input.deck_bytes);
    let mut state = CutthroatState::new_with_deck(dealer, deck.clone());

    let mut tokenlog = encode_header(dealer, &deck);
    let mut generated_actions = Vec::new();

    let mut step = 0usize;
    loop {
        if matches!(state.phase, Phase::GameOver) {
            break;
        }

        let seat = acting_seat(&state);

        let legal = state.legal_actions(seat);
        assert!(
            !legal.is_empty(),
            "non-terminal phase has no legal actions: phase={:?} seat={}",
            state.phase,
            seat
        );

        maybe_reject_illegal_action(&mut state, seat, &legal, &input, step);

        let index = usize::from(sample_byte(&input.move_bytes, step)) % legal.len();
        let action = legal[index].clone();
        let state_before = state.clone();

        state
            .apply(seat, action.clone())
            .expect("selected legal action should always apply");
        append_action(&mut tokenlog, &state_before, seat, &action)
            .expect("valid seat should always append to tokenlog");
        generated_actions.push((seat, action));
        maybe_assert_roundtrip_via_token_vec(&tokenlog, &state, &input, step);
        step = step.wrapping_add(1);
    }

    let parsed = parse_tokenlog(&tokenlog).expect("generated tokenlog should parse");
    assert_eq!(parsed.dealer, dealer);
    assert_eq!(parsed.deck, deck);
    assert_eq!(parsed.actions.len(), generated_actions.len());
    assert_eq!(parsed.actions, generated_actions);

    let mut rebuilt = encode_header(parsed.dealer, &parsed.deck);
    let mut rebuild_state = CutthroatState::new_with_deck(parsed.dealer, parsed.deck.clone());
    for (seat, action) in &parsed.actions {
        append_action(&mut rebuilt, &rebuild_state, *seat, action).expect("parsed action should re-encode");
        rebuild_state
            .apply(*seat, action.clone())
            .expect("parsed action should apply while rebuilding");
    }
    assert_eq!(rebuilt, tokenlog);

    let replayed = replay_tokenlog(&parsed).expect("parsed tokenlog should replay");
    assert_eq!(state_digest(&replayed), state_digest(&state));
}

fn maybe_reject_illegal_action(
    state: &mut CutthroatState,
    seat: u8,
    legal: &[Action],
    input: &FuzzInput,
    step: usize,
) {
    let roll = sample_byte(&input.move_bytes, step.wrapping_mul(17).wrapping_add(11));
    if roll % 50 != 0 {
        return;
    }

    let illegal = pick_illegal_action(
        state,
        legal,
        sample_byte(&input.move_bytes, step.wrapping_mul(17).wrapping_add(23)),
    );
    assert!(
        !legal.contains(&illegal),
        "illegal probe unexpectedly selected legal action: {illegal:?}"
    );

    let before = state_digest(state);
    let err = state
        .apply(seat, illegal)
        .expect_err("injected illegal action must be rejected");
    assert!(
        matches!(err, RuleError::IllegalAction),
        "expected IllegalAction for injected illegal move, got {err:?}"
    );
    assert_eq!(
        before,
        state_digest(state),
        "state changed after rejected illegal action"
    );
}

fn maybe_assert_roundtrip_via_token_vec(
    tokenlog: &str,
    expected_state: &CutthroatState,
    input: &FuzzInput,
    step: usize,
) {
    let roll = sample_byte(&input.move_bytes, step.wrapping_mul(17).wrapping_add(41));
    if roll % 20 != 0 {
        return;
    }
    assert_state_roundtrip_via_token_vec(tokenlog, expected_state);
}

fn pick_illegal_action(state: &CutthroatState, legal: &[Action], selector: u8) -> Action {
    let mut candidates = match &state.phase {
        Phase::Main => vec![
            Action::CounterPass,
            Action::Draw,
            Action::PlayOneOff {
                card: Card::Joker(0),
                target: OneOffTarget::None,
            },
        ],
        Phase::Countering(_) => vec![
            Action::Draw,
            Action::Pass,
            Action::PlayPoints {
                card: Card::Joker(0),
            },
        ],
        Phase::ResolvingThree { .. } => vec![
            Action::Pass,
            Action::Draw,
            Action::ResolveThreePick {
                card_from_scrap: Card::Joker(0),
            },
        ],
        Phase::ResolvingFour { .. } => vec![
            Action::Pass,
            Action::Draw,
            Action::ResolveFourDiscard {
                card: Card::Joker(0),
            },
        ],
        Phase::ResolvingFive { .. } => vec![
            Action::Pass,
            Action::Draw,
            Action::ResolveFiveDiscard {
                card: Card::Joker(0),
            },
        ],
        Phase::ResolvingSeven { .. } => vec![
            Action::Pass,
            Action::Draw,
            Action::ResolveSevenChoose {
                card: Card::Joker(0),
                play: SevenPlay::Discard,
            },
        ],
        Phase::GameOver => vec![Action::Draw],
    };

    candidates.extend([
        Action::CounterTwo {
            two_card: Card::Joker(0),
        },
        Action::PlayPoints {
            card: Card::Joker(0),
        },
        Action::PlayRoyal {
            card: Card::Joker(0),
        },
        Action::PlayJack {
            jack: Card::Joker(0),
            target_point_base: Card::Joker(1),
        },
        Action::PlayJoker {
            joker: Card::Joker(0),
            target_royal_card: Card::Joker(1),
        },
        Action::Scuttle {
            card: Card::Joker(0),
            target_point_base: Card::Joker(1),
        },
    ]);

    let len = candidates.len();
    for i in 0..len {
        let idx = (usize::from(selector) + i) % len;
        let candidate = candidates[idx].clone();
        if !legal.contains(&candidate) {
            return candidate;
        }
    }

    // Defensive fallback; there should always be an illegal action available.
    Action::PlayPoints {
        card: Card::Joker(0),
    }
}

fn assert_state_roundtrip_via_token_vec(tokenlog: &str, expected_state: &CutthroatState) {
    let token_vec =
        parse_token_slice(tokenlog).expect("generated tokenlog should tokenize into typed token vec");
    let reparsed = parse_tokenlog(&join_tokens(&token_vec)).expect("token vec should parse as tokenlog");
    let replayed = replay_tokenlog(&reparsed).expect("reparsed tokenlog should replay");
    assert_eq!(state_digest(&replayed), state_digest(expected_state));
}

fn shuffled_deck(deck_bytes: &[u8]) -> Vec<Card> {
    let mut deck = full_deck_with_jokers();
    for i in (1..deck.len()).rev() {
        let swap_idx = usize::from(sample_byte(deck_bytes, deck.len() - 1 - i)) % (i + 1);
        deck.swap(i, swap_idx);
    }
    deck
}

fn sample_byte(bytes: &[u8], idx: usize) -> u8 {
    if bytes.is_empty() {
        0
    } else {
        bytes[idx % bytes.len()]
    }
}

fn acting_seat(state: &CutthroatState) -> u8 {
    match &state.phase {
        Phase::Main => state.turn,
        Phase::Countering(counter) => counter.next_seat,
        Phase::ResolvingThree { seat, .. }
        | Phase::ResolvingFour { seat, .. }
        | Phase::ResolvingFive { seat, .. }
        | Phase::ResolvingSeven { seat, .. } => *seat,
        Phase::GameOver => panic!("acting_seat requested for game-over phase"),
    }
}

fn state_digest(state: &CutthroatState) -> StateDigest {
    StateDigest {
        dealer: state.dealer,
        turn: state.turn,
        phase: phase_digest(&state.phase),
        deck: state.deck.clone(),
        scrap: state.scrap.clone(),
        players: state
            .players
            .iter()
            .map(|player| PlayerDigest {
                hand: player.hand.clone(),
                points: player
                    .points
                    .iter()
                    .map(|stack| PointStackDigest {
                        base: stack.base,
                        base_owner: stack.base_owner,
                        jacks: stack.jacks.iter().map(|jack| (jack.card, jack.owner)).collect(),
                    })
                    .collect(),
                royals: player
                    .royals
                    .iter()
                    .map(|stack| RoyalStackDigest {
                        base: stack.base,
                        base_owner: stack.base_owner,
                        jokers: stack
                            .jokers
                            .iter()
                            .map(|joker| (joker.card, joker.owner))
                            .collect(),
                    })
                    .collect(),
                frozen: player
                    .frozen
                    .iter()
                    .map(|frozen| (frozen.card, frozen.remaining_turns))
                    .collect(),
            })
            .collect(),
        pass_streak_start: state.pass_streak_start,
        pass_streak_len: state.pass_streak_len,
        winner: state.winner.clone(),
    }
}

fn phase_digest(phase: &Phase) -> PhaseDigest {
    match phase {
        Phase::Main => PhaseDigest::Main,
        Phase::Countering(counter) => PhaseDigest::Countering {
            base_player: counter.base_player,
            oneoff: counter.oneoff.clone(),
            twos: counter.twos.clone(),
            next_seat: counter.next_seat,
            rotation_anchor: counter.rotation_anchor,
        },
        Phase::ResolvingThree { seat, base_player } => PhaseDigest::ResolvingThree {
            seat: *seat,
            base_player: *base_player,
        },
        Phase::ResolvingFour {
            seat,
            base_player,
            remaining,
        } => PhaseDigest::ResolvingFour {
            seat: *seat,
            base_player: *base_player,
            remaining: *remaining,
        },
        Phase::ResolvingFive {
            seat,
            base_player,
            discarded,
        } => PhaseDigest::ResolvingFive {
            seat: *seat,
            base_player: *base_player,
            discarded: *discarded,
        },
        Phase::ResolvingSeven {
            seat,
            base_player,
            revealed,
        } => PhaseDigest::ResolvingSeven {
            seat: *seat,
            base_player: *base_player,
            revealed: revealed.clone(),
        },
        Phase::GameOver => PhaseDigest::GameOver,
    }
}
