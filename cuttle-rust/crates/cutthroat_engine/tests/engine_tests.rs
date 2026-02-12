use cutthroat_engine::state::{
    FrozenCard, HAND_LIMIT, JackOnStack, JokerOnStack, PointStack, PublicCard, RoyalStack,
};
use cutthroat_engine::{
    Action, Card, CutthroatState, OneOffTarget, Phase, RuleError, SevenPlay, Winner, append_action,
    encode_header, full_deck_with_jokers, parse_tokenlog, replay_tokenlog,
};

fn c(token: &str) -> Card {
    Card::from_token(token).expect("card token")
}

fn assert_tokenlog_roundtrip(tokens: &str) -> CutthroatState {
    let log = parse_tokenlog(tokens).unwrap();
    let mut rebuilt = encode_header(log.dealer, &log.deck);
    let mut rebuild_state = CutthroatState::new_with_deck(log.dealer, log.deck.clone());
    for (seat, action) in &log.actions {
        append_action(&mut rebuilt, &rebuild_state, *seat, action).unwrap();
        rebuild_state.apply(*seat, action.clone()).unwrap();
    }
    assert_eq!(rebuilt, tokens);

    let mut state = CutthroatState::new_with_deck(log.dealer, log.deck.clone());
    for (idx, (seat, action)) in log.actions.iter().enumerate() {
        if let Err(err) = state.apply(*seat, action.clone()) {
            let legal = state.legal_actions(*seat);
            panic!("apply failed at {idx} seat {seat} action {action:?}: {err:?}. legal={legal:?}");
        }
    }
    state
}

fn empty_state() -> CutthroatState {
    let deck = full_deck_with_jokers();
    let mut state = CutthroatState::new_with_deck(2, deck);
    for player in &mut state.players {
        player.hand.clear();
        player.points.clear();
        player.royals.clear();
        player.frozen.clear();
    }
    state.scrap.clear();
    state.deck.clear();
    state
}

fn build_tokenlog_from_script(
    dealer: u8,
    deck_tokens: &[&str],
    script: &[(u8, Action)],
) -> (String, CutthroatState) {
    let deck: Vec<Card> = deck_tokens.iter().map(|token| c(token)).collect();
    let mut state = CutthroatState::new_with_deck(dealer, deck.clone());
    let mut tokenlog = encode_header(dealer, &deck);

    for (index, (seat, action)) in script.iter().enumerate() {
        let legal = state.legal_actions(*seat);
        assert!(
            legal.contains(action),
            "script action at index {index} is not legal for seat {seat}: {action:?}. legal={legal:?}"
        );
        append_action(&mut tokenlog, &state, *seat, action).expect("append_action should succeed");
        state
            .apply(*seat, action.clone())
            .expect("script action should apply");
    }

    (tokenlog, state)
}

#[test]
fn stalemate_after_four_passes_same_start() {
    let mut state = empty_state();
    state.deck.clear();
    state.turn = 0;

    state.apply(0, Action::Pass).unwrap();
    assert!(state.winner.is_none());
    state.apply(1, Action::Pass).unwrap();
    assert!(state.winner.is_none());
    state.apply(2, Action::Pass).unwrap();
    assert!(state.winner.is_none());
    state.apply(0, Action::Pass).unwrap();

    assert_eq!(state.winner, Some(Winner::Draw));
}

#[test]
fn king_threshold_win() {
    let mut state = empty_state();
    state.turn = 0;
    state.players[0].points.push(PointStack {
        base: c("9C"),
        base_owner: 0,
        jacks: Vec::new(),
    });
    state.players[0].hand.push(c("KH"));

    state.apply(0, Action::PlayRoyal { card: c("KH") }).unwrap();
    assert_eq!(state.winner, Some(Winner::Seat(0)));
}

#[test]
fn joker_stacking_and_revert() {
    let mut state = empty_state();
    state.turn = 0;
    state.players[0].hand.push(Card::Joker(0));
    state.players[2].hand.push(Card::Joker(1));
    state.players[1].hand.push(c("2C"));

    state.players[1].royals.push(RoyalStack {
        base: c("KC"),
        base_owner: 1,
        jokers: Vec::new(),
    });

    state
        .apply(
            0,
            Action::PlayJoker {
                joker: Card::Joker(0),
                target_royal_card: c("KC"),
            },
        )
        .unwrap();
    assert_eq!(state.players[0].royals.len(), 1);
    assert_eq!(state.players[2].royals.len(), 0);

    state.turn = 2;
    state
        .apply(
            2,
            Action::PlayJoker {
                joker: Card::Joker(1),
                target_royal_card: c("KC"),
            },
        )
        .unwrap();
    assert_eq!(state.players[2].royals.len(), 1);

    state.turn = 1;
    state
        .apply(
            1,
            Action::PlayOneOff {
                card: c("2C"),
                target: OneOffTarget::Joker {
                    card: Card::Joker(1),
                },
            },
        )
        .unwrap();
    state.apply(2, Action::CounterPass).unwrap();
    state.apply(0, Action::CounterPass).unwrap();

    assert_eq!(state.players[0].royals.len(), 1);
    assert_eq!(state.players[2].royals.len(), 0);
}

#[test]
fn queen_protection_blocks_joker_targets() {
    let mut state = empty_state();
    state.turn = 0;
    state.players[0].hand.push(Card::Joker(0));
    state.players[1].royals.push(RoyalStack {
        base: c("QH"),
        base_owner: 1,
        jokers: Vec::new(),
    });
    state.players[1].royals.push(RoyalStack {
        base: c("KH"),
        base_owner: 1,
        jokers: Vec::new(),
    });

    let legal = state.legal_actions(0);
    assert!(legal.contains(&Action::PlayJoker {
        joker: Card::Joker(0),
        target_royal_card: c("QH")
    }));
    assert!(!legal.contains(&Action::PlayJoker {
        joker: Card::Joker(0),
        target_royal_card: c("KH")
    }));
}

#[test]
fn joker_can_steal_top_jack_and_move_point_stack() {
    let mut state = empty_state();
    state.turn = 0;
    state.players[0].hand.push(Card::Joker(0));
    state.players[1].points.push(PointStack {
        base: c("5C"),
        base_owner: 1,
        jacks: vec![JackOnStack {
            card: c("JD"),
            owner: 2,
        }],
    });

    let legal = state.legal_actions(0);
    assert!(legal.contains(&Action::PlayJoker {
        joker: Card::Joker(0),
        target_royal_card: c("JD"),
    }));

    state
        .apply(
            0,
            Action::PlayJoker {
                joker: Card::Joker(0),
                target_royal_card: c("JD"),
            },
        )
        .unwrap();

    assert!(state.players[1].points.is_empty());
    assert_eq!(state.players[0].points.len(), 1);
    let stolen = &state.players[0].points[0];
    assert_eq!(stolen.base, c("5C"));
    assert_eq!(stolen.jacks.len(), 2);
    assert_eq!(
        stolen.jacks.last().map(|jack| jack.card),
        Some(Card::Joker(0))
    );
    assert_eq!(stolen.controller(), 0);
}

#[test]
fn seven_joker_can_target_top_jack() {
    let mut state = empty_state();
    state.turn = 0;
    state.phase = Phase::ResolvingSeven {
        seat: 0,
        base_player: 0,
        revealed: vec![Card::Joker(0)],
    };
    state.players[1].points.push(PointStack {
        base: c("5C"),
        base_owner: 1,
        jacks: vec![JackOnStack {
            card: c("JD"),
            owner: 2,
        }],
    });

    let legal = state.legal_actions(0);
    assert!(legal.contains(&Action::ResolveSevenChoose {
        card: Card::Joker(0),
        play: SevenPlay::Joker { target: c("JD") },
    }));

    state
        .apply(
            0,
            Action::ResolveSevenChoose {
                card: Card::Joker(0),
                play: SevenPlay::Joker { target: c("JD") },
            },
        )
        .unwrap();

    assert!(state.players[1].points.is_empty());
    assert_eq!(state.players[0].points.len(), 1);
    assert_eq!(
        state.players[0].points[0]
            .jacks
            .last()
            .map(|jack| jack.card),
        Some(Card::Joker(0))
    );
}

#[test]
fn countering_parity_fizzles() {
    let mut state = empty_state();
    state.turn = 0;
    state.players[0].hand.push(c("AC"));
    state.players[1].hand.push(c("2D"));

    state
        .apply(
            0,
            Action::PlayOneOff {
                card: c("AC"),
                target: OneOffTarget::None,
            },
        )
        .unwrap();
    state
        .apply(1, Action::CounterTwo { two_card: c("2D") })
        .unwrap();
    state.apply(2, Action::CounterPass).unwrap();
    state.apply(0, Action::CounterPass).unwrap();

    assert!(state.scrap.contains(&c("AC")));
    assert!(state.scrap.contains(&c("2D")));
}

#[test]
fn seven_reveal_play_and_return() {
    let mut state = empty_state();
    state.turn = 0;
    state.players[0].hand.push(c("7C"));
    state.deck = vec![c("5C"), c("KD")];

    state
        .apply(
            0,
            Action::PlayOneOff {
                card: c("7C"),
                target: OneOffTarget::None,
            },
        )
        .unwrap();
    state.apply(1, Action::CounterPass).unwrap();
    state.apply(2, Action::CounterPass).unwrap();

    state
        .apply(
            0,
            Action::ResolveSevenChoose {
                card: c("5C"),
                play: SevenPlay::Points,
            },
        )
        .unwrap();

    assert_eq!(state.players[0].points.len(), 1);
    assert_eq!(state.players[0].points[0].base, c("5C"));
    assert_eq!(state.deck.first().copied(), Some(c("KD")));
}

#[test]
fn seven_oneoff_enters_countering() {
    let mut state = empty_state();
    state.turn = 0;
    state.players[0].hand.push(c("7C"));
    state.deck = vec![c("AC"), c("KD")];

    state
        .apply(
            0,
            Action::PlayOneOff {
                card: c("7C"),
                target: OneOffTarget::None,
            },
        )
        .unwrap();
    state.apply(1, Action::CounterPass).unwrap();
    state.apply(2, Action::CounterPass).unwrap();

    state
        .apply(
            0,
            Action::ResolveSevenChoose {
                card: c("AC"),
                play: SevenPlay::OneOff {
                    target: OneOffTarget::None,
                },
            },
        )
        .unwrap();

    assert!(matches!(state.phase, Phase::Countering(_)));
}

#[test]
fn resolving_seven_revealed_cards_visible_only_to_resolver() {
    let mut state = empty_state();
    state.turn = 0;
    state.players[0].hand.push(c("7C"));
    state.deck = vec![c("5C"), c("KD")];

    state
        .apply(
            0,
            Action::PlayOneOff {
                card: c("7C"),
                target: OneOffTarget::None,
            },
        )
        .unwrap();
    state.apply(1, Action::CounterPass).unwrap();
    state.apply(2, Action::CounterPass).unwrap();

    let view_resolver = state.public_view(0);
    let view_other = state.public_view(1);

    match view_resolver.phase {
        cutthroat_engine::state::PhaseView::ResolvingSeven { revealed_cards, .. } => {
            assert_eq!(revealed_cards, vec!["5C".to_string(), "KD".to_string()]);
        }
        _ => panic!("expected resolving seven for resolver"),
    }

    match view_other.phase {
        cutthroat_engine::state::PhaseView::ResolvingSeven { revealed_cards, .. } => {
            assert!(revealed_cards.is_empty());
        }
        _ => panic!("expected resolving seven for other player"),
    }
}

#[test]
fn tokenlog_roundtrip_replay() {
    let mut deck = full_deck_with_jokers();
    deck.retain(|card| *card != c("AC"));
    deck.insert(0, c("AC"));
    let mut tokens = encode_header(2, &deck);
    let state = CutthroatState::new_with_deck(2, deck.clone());
    append_action(
        &mut tokens,
        &state,
        0,
        &Action::PlayPoints { card: c("AC") },
    )
    .unwrap();

    let log = parse_tokenlog(&tokens).unwrap();
    let state = replay_tokenlog(&log).unwrap();

    assert_eq!(state.players[0].points.len(), 1);
    assert_eq!(state.players[0].points[0].base, c("AC"));
}

#[test]
fn full_game_tokenlog_roundtrip_and_replay() {
    const TOKENLOG: &str = concat!(
        "V1 CUTTHROAT3P DEALER P2 DECK ",
        "KC 9C 7C KD 3D 3H KH 4D 4H AC 5D 5H 2C 6D 6H 3C 4C 5C 6C 8C TC JC QC AD 2D 7D 8D 9D TD JD QD AH 2H 7H 8H 9H TH JH QH AS 2S 3S 4S 5S 6S 7S 8S 9S TS JS QS KS J0 J1 ",
        "ENDDECK ",
        "P0 playRoyal KC ",
        "P1 points 9C ",
        "P2 points 7C ",
        "P0 playRoyal KD ",
        "P1 draw 3C ",
        "P2 draw 4C ",
        "P0 playRoyal KH"
    );

    let state = assert_tokenlog_roundtrip(TOKENLOG);
    assert_eq!(state.winner, Some(Winner::Seat(0)));
}

#[test]
fn full_game_tokenlog_game_two_exercises_oneoffs() {
    let deck = [
        "3C", "2D", "5H", "7C", "4D", "7H", "5C", "6D", "9H", "4C", "8D", "QH", "9C", "JD", "J0",
        "AC", "2C", "6C", "8C", "TC", "JC", "QC", "KC", "AD", "3D", "5D", "7D", "9D", "TD", "JD",
        "QD", "AH", "2H", "7H", "8H", "9H", "TH", "JH", "QH", "AS", "2S", "3S", "4S", "5S", "6S",
        "7S", "8S", "9S", "TS", "JS", "QS", "KS", "J1",
    ];
    let script = vec![
        (
            0,
            Action::PlayOneOff {
                card: c("3C"),
                target: OneOffTarget::None,
            },
        ),
        (1, Action::CounterPass),
        (2, Action::CounterPass),
        (1, Action::PlayPoints { card: c("6D") }),
        (2, Action::PlayPoints { card: c("5H") }),
        (
            0,
            Action::Scuttle {
                card: c("9C"),
                target_point_base: c("6D"),
            },
        ),
        (
            1,
            Action::PlayOneOff {
                card: c("4D"),
                target: OneOffTarget::Player { seat: 2 },
            },
        ),
        (2, Action::CounterPass),
        (0, Action::CounterPass),
        (2, Action::ResolveFourDiscard { card: c("QH") }),
        (2, Action::ResolveFourDiscard { card: c("7H") }),
        (2, Action::PlayPoints { card: c("9H") }),
    ];
    assert!(script.len() >= 12, "script unexpectedly short");
    let (tokenlog, expected_state) = build_tokenlog_from_script(2, &deck, &script);
    let state = assert_tokenlog_roundtrip(&tokenlog);
    assert_eq!(state, expected_state);
}

#[test]
fn full_game_tokenlog_game_three_exercises_counters_and_resolves() {
    let deck = [
        "QH", "5H", "6H", "KH", "2S", "6C", "9D", "7C", "7D", "4H", "4C", "3C", "2H", "8D", "2C",
        "AC", "3D", "8C", "9C", "5C", "TC", "JC", "QC", "KC", "AD", "2D", "4D", "5D", "6D", "TD",
        "JD", "QD", "KD", "AH", "3H", "7H", "8H", "9H", "TH", "JH", "AS", "3S", "4S", "5S", "6S",
        "7S", "8S", "9S", "TS", "JS", "QS", "KS", "J0", "J1",
    ];
    let script = vec![
        (1, Action::PlayRoyal { card: c("QH") }),
        (2, Action::PlayPoints { card: c("4C") }),
        (0, Action::PlayPoints { card: c("6C") }),
        (1, Action::PlayRoyal { card: c("KH") }),
        (
            2,
            Action::PlayOneOff {
                card: c("7C"),
                target: OneOffTarget::None,
            },
        ),
        (0, Action::CounterPass),
        (1, Action::CounterPass),
        (
            2,
            Action::ResolveSevenChoose {
                card: c("AC"),
                play: SevenPlay::OneOff {
                    target: OneOffTarget::None,
                },
            },
        ),
        (0, Action::CounterPass),
        (1, Action::CounterPass),
        (
            0,
            Action::PlayOneOff {
                card: c("6H"),
                target: OneOffTarget::None,
            },
        ),
        (1, Action::CounterTwo { two_card: c("2H") }),
        (2, Action::CounterTwo { two_card: c("2S") }),
        (0, Action::CounterPass),
        (1, Action::CounterPass),
        (
            1,
            Action::PlayOneOff {
                card: c("4H"),
                target: OneOffTarget::Player { seat: 0 },
            },
        ),
        (2, Action::CounterPass),
        (0, Action::CounterPass),
        (0, Action::ResolveFourDiscard { card: c("7D") }),
        (0, Action::ResolveFourDiscard { card: c("2C") }),
        (
            2,
            Action::PlayOneOff {
                card: c("5H"),
                target: OneOffTarget::None,
            },
        ),
        (0, Action::CounterPass),
        (1, Action::CounterPass),
        (2, Action::ResolveFiveDiscard { card: c("8D") }),
        (0, Action::PlayPoints { card: c("3C") }),
        (
            1,
            Action::PlayOneOff {
                card: c("9D"),
                target: OneOffTarget::Point { base: c("3C") },
            },
        ),
        (2, Action::CounterPass),
        (0, Action::CounterPass),
    ];
    assert!(script.len() >= 24, "script unexpectedly short");
    let (tokenlog, expected_state) = build_tokenlog_from_script(0, &deck, &script);
    let state = assert_tokenlog_roundtrip(&tokenlog);
    assert_eq!(state.winner, expected_state.winner);
    assert!(state.players.iter().all(|p| p.points.is_empty()));
    assert!(state.players.iter().all(|p| p.royals.is_empty()));
    assert!(state.players.iter().any(|player| !player.frozen.is_empty()));
}

#[test]
fn full_game_tokenlog_game_four_targets_and_wipes() {
    let deck = [
        "5C", "JD", "2H", "KH", "J0", "2S", "6D", "AC", "3C", "4C", "7C", "8C", "9C", "QD", "KC",
        "2C", "6C", "TC", "JC", "QC", "AD", "2D", "3D", "4D", "5D", "7D", "8D", "9D", "TD", "KD",
        "AH", "3H", "4H", "5H", "6H", "7H", "8H", "9H", "TH", "JH", "QH", "AS", "3S", "4S", "5S",
        "6S", "7S", "8S", "9S", "TS", "JS", "QS", "KS", "J1",
    ];
    let script = vec![
        (0, Action::PlayPoints { card: c("5C") }),
        (
            1,
            Action::PlayJack {
                jack: c("JD"),
                target_point_base: c("5C"),
            },
        ),
        (
            2,
            Action::PlayOneOff {
                card: c("2H"),
                target: OneOffTarget::Jack { card: c("JD") },
            },
        ),
        (0, Action::CounterPass),
        (1, Action::CounterPass),
        (0, Action::PlayRoyal { card: c("KH") }),
        (
            1,
            Action::PlayJoker {
                joker: Card::Joker(0),
                target_royal_card: c("KH"),
            },
        ),
        (
            2,
            Action::PlayOneOff {
                card: c("2S"),
                target: OneOffTarget::Joker {
                    card: Card::Joker(0),
                },
            },
        ),
        (0, Action::CounterPass),
        (1, Action::CounterPass),
        (
            0,
            Action::PlayOneOff {
                card: c("6D"),
                target: OneOffTarget::None,
            },
        ),
        (1, Action::CounterPass),
        (2, Action::CounterPass),
        (
            1,
            Action::PlayOneOff {
                card: c("AC"),
                target: OneOffTarget::None,
            },
        ),
        (2, Action::CounterPass),
        (0, Action::CounterPass),
        (2, Action::Draw),
    ];
    assert!(script.len() >= 16, "script unexpectedly short");
    let (tokenlog, expected_state) = build_tokenlog_from_script(2, &deck, &script);
    let state = assert_tokenlog_roundtrip(&tokenlog);
    assert_eq!(state.winner, expected_state.winner);
    assert!(state.players.iter().all(|p| p.points.is_empty()));
    assert!(state.players.iter().all(|p| p.royals.is_empty()));
}

#[test]
fn full_game_tokenlog_game_five_seven_variants() {
    let deck = [
        "3C", "4C", "QH", "7C", "7D", "7H", "2C", "5C", "9C", "6D", "6C", "KC", "8S", "8D", "AD",
        "JH", "J0", "9H", "5S", "AC", "8C", "TC", "JC", "QC", "2D", "3D", "4D", "5D", "9D", "TD",
        "JD", "QD", "KD", "AH", "2H", "3H", "4H", "5H", "6H", "8H", "TH", "KH", "AS", "2S", "3S",
        "4S", "6S", "7S", "9S", "TS", "JS", "QS", "KS", "J1",
    ];
    let script = vec![
        (0, Action::PlayPoints { card: c("3C") }),
        (1, Action::PlayPoints { card: c("4C") }),
        (2, Action::PlayRoyal { card: c("QH") }),
        (
            0,
            Action::PlayOneOff {
                card: c("7C"),
                target: OneOffTarget::None,
            },
        ),
        (1, Action::CounterPass),
        (2, Action::CounterPass),
        (
            0,
            Action::ResolveSevenChoose {
                card: c("JH"),
                play: SevenPlay::Jack { target: c("4C") },
            },
        ),
        (
            1,
            Action::PlayOneOff {
                card: c("7D"),
                target: OneOffTarget::None,
            },
        ),
        (2, Action::CounterPass),
        (0, Action::CounterPass),
        (
            1,
            Action::ResolveSevenChoose {
                card: Card::Joker(0),
                play: SevenPlay::Joker { target: c("QH") },
            },
        ),
        (
            2,
            Action::PlayOneOff {
                card: c("7H"),
                target: OneOffTarget::None,
            },
        ),
        (0, Action::CounterPass),
        (1, Action::CounterPass),
        (
            2,
            Action::ResolveSevenChoose {
                card: c("9H"),
                play: SevenPlay::Scuttle { target: c("4C") },
            },
        ),
    ];
    assert!(script.len() >= 15, "script unexpectedly short");
    let (tokenlog, expected_state) = build_tokenlog_from_script(2, &deck, &script);
    let state = assert_tokenlog_roundtrip(&tokenlog);
    assert_eq!(state, expected_state);
}

#[test]
fn full_game_tokenlog_stalemate_only() {
    const TOKENLOG: &str = concat!(
        "V1 CUTTHROAT3P DEALER P0 DECK ",
        "AC AD AH 2C 2D 2H 3C 3D 3H 4C 4D 4H 5C 5D 5H ",
        "ENDDECK ",
        "P1 pass ",
        "P2 pass ",
        "P0 pass ",
        "P1 pass"
    );

    let state = assert_tokenlog_roundtrip(TOKENLOG);
    assert_eq!(state.winner, Some(Winner::Draw));
}

#[test]
fn seven_double_jack_no_points_only_discard() {
    let mut state = empty_state();
    state.turn = 0;
    state.players[0].hand.push(c("7C"));
    state.deck = vec![c("JH"), c("JD")];

    state
        .apply(
            0,
            Action::PlayOneOff {
                card: c("7C"),
                target: OneOffTarget::None,
            },
        )
        .unwrap();
    state.apply(1, Action::CounterPass).unwrap();
    state.apply(2, Action::CounterPass).unwrap();

    let legal = state.legal_actions(0);
    assert_eq!(legal.len(), 2);
    assert!(legal.contains(&Action::ResolveSevenChoose {
        card: c("JH"),
        play: SevenPlay::Discard
    }));
    assert!(legal.contains(&Action::ResolveSevenChoose {
        card: c("JD"),
        play: SevenPlay::Discard
    }));

    state
        .apply(
            0,
            Action::ResolveSevenChoose {
                card: c("JH"),
                play: SevenPlay::Discard,
            },
        )
        .unwrap();
    assert!(state.scrap.contains(&c("JH")));
    assert_eq!(state.deck.first().copied(), Some(c("JD")));
}

#[test]
fn counter_rotation_end_after_last_two_even_resolves() {
    let mut state = empty_state();
    state.turn = 0;
    state.players[0].hand.push(c("AC"));
    state.players[1].hand.push(c("2C"));
    state.players[2].hand.push(c("2D"));
    state.players[1].points.push(PointStack {
        base: c("9C"),
        base_owner: 1,
        jacks: Vec::new(),
    });

    state
        .apply(
            0,
            Action::PlayOneOff {
                card: c("AC"),
                target: OneOffTarget::None,
            },
        )
        .unwrap();
    state
        .apply(1, Action::CounterTwo { two_card: c("2C") })
        .unwrap();
    state
        .apply(2, Action::CounterTwo { two_card: c("2D") })
        .unwrap();
    state.apply(0, Action::CounterPass).unwrap();
    state.apply(1, Action::CounterPass).unwrap();

    assert!(state.players.iter().all(|p| p.points.is_empty()));
    assert!(matches!(state.phase, Phase::Main));
}

#[test]
fn counter_rotation_end_after_last_two_odd_fizzles() {
    let mut state = empty_state();
    state.turn = 0;
    state.players[0].hand.push(c("AC"));
    state.players[1].hand.push(c("2C"));
    state.players[2].hand.push(c("2D"));
    state.players[1].points.push(PointStack {
        base: c("9C"),
        base_owner: 1,
        jacks: Vec::new(),
    });

    state
        .apply(
            0,
            Action::PlayOneOff {
                card: c("AC"),
                target: OneOffTarget::None,
            },
        )
        .unwrap();
    state
        .apply(1, Action::CounterTwo { two_card: c("2C") })
        .unwrap();
    state.apply(2, Action::CounterPass).unwrap();
    state.apply(0, Action::CounterPass).unwrap();

    assert!(state.players[1].points.iter().any(|p| p.base == c("9C")));
    assert!(matches!(state.phase, Phase::Main));
}

#[test]
fn resolve_five_draws_respects_hand_limit() {
    let mut state = empty_state();
    state.turn = 0;
    state.phase = Phase::ResolvingFive {
        seat: 0,
        base_player: 0,
        discarded: false,
    };
    state.players[0].hand = vec![
        c("5C"),
        c("AD"),
        c("KD"),
        c("QD"),
        c("JD"),
        c("TD"),
        c("9D"),
    ];
    state.deck = vec![c("AC"), c("KC"), c("QC")];

    state
        .apply(0, Action::ResolveFiveDiscard { card: c("5C") })
        .unwrap();
    assert_eq!(state.players[0].hand.len(), HAND_LIMIT);
    assert!(state.players[0].hand.contains(&c("AC")));
    assert_eq!(state.deck, vec![c("KC"), c("QC")]);
}

#[test]
fn frozen_card_not_usable_in_legal_actions() {
    let mut state = empty_state();
    state.turn = 0;
    state.players[0].hand.push(c("9C"));
    state.players[0].frozen.push(FrozenCard {
        card: c("9C"),
        remaining_turns: 1,
    });

    let legal = state.legal_actions(0);
    assert!(!legal.contains(&Action::PlayPoints { card: c("9C") }));
    assert!(!legal.contains(&Action::PlayOneOff {
        card: c("9C"),
        target: OneOffTarget::None
    }));
}

#[test]
fn nine_returns_top_jack_only() {
    let mut state = empty_state();
    state.turn = 0;
    state.players[0].hand.push(c("9C"));
    state.players[1].points.push(PointStack {
        base: c("5C"),
        base_owner: 1,
        jacks: vec![
            JackOnStack {
                card: c("JD"),
                owner: 0,
            },
            JackOnStack {
                card: c("JH"),
                owner: 2,
            },
        ],
    });

    state
        .apply(
            0,
            Action::PlayOneOff {
                card: c("9C"),
                target: OneOffTarget::Jack { card: c("JH") },
            },
        )
        .unwrap();
    state.apply(1, Action::CounterPass).unwrap();
    state.apply(2, Action::CounterPass).unwrap();

    assert!(state.players[2].hand.contains(&c("JH")));
    assert!(state.players[0].points.iter().any(|p| p.base == c("5C")));
}

#[test]
fn resolve_five_draws_one_card_left() {
    let mut state = empty_state();
    state.turn = 0;
    state.phase = Phase::ResolvingFive {
        seat: 0,
        base_player: 0,
        discarded: false,
    };
    state.players[0].hand = vec![c("5H"), c("AD"), c("KD"), c("QD")];
    state.deck = vec![c("AC")];

    state
        .apply(0, Action::ResolveFiveDiscard { card: c("5H") })
        .unwrap();
    assert_eq!(state.players[0].hand.len(), 4);
    assert!(state.players[0].hand.contains(&c("AC")));
    assert!(state.deck.is_empty());
}

#[test]
fn resolve_five_draws_two_cards_left() {
    let mut state = empty_state();
    state.turn = 0;
    state.phase = Phase::ResolvingFive {
        seat: 0,
        base_player: 0,
        discarded: false,
    };
    state.players[0].hand = vec![c("5D"), c("AD"), c("KD"), c("QD")];
    state.deck = vec![c("AC"), c("KC")];

    state
        .apply(0, Action::ResolveFiveDiscard { card: c("5D") })
        .unwrap();
    assert_eq!(state.players[0].hand.len(), 5);
    assert!(state.players[0].hand.contains(&c("AC")));
    assert!(state.players[0].hand.contains(&c("KC")));
    assert!(state.deck.is_empty());
}

#[test]
fn jack_blocked_by_queen_protection() {
    let mut state = empty_state();
    state.turn = 0;
    state.players[0].hand.push(c("JD"));
    state.players[1].points.push(PointStack {
        base: c("9C"),
        base_owner: 1,
        jacks: Vec::new(),
    });
    state.players[1].royals.push(RoyalStack {
        base: c("QH"),
        base_owner: 1,
        jokers: Vec::new(),
    });

    let legal = state.legal_actions(0);
    assert!(!legal.contains(&Action::PlayJack {
        jack: c("JD"),
        target_point_base: c("9C")
    }));
    let err = state
        .apply(
            0,
            Action::PlayJack {
                jack: c("JD"),
                target_point_base: c("9C"),
            },
        )
        .unwrap_err();
    assert!(matches!(err, RuleError::IllegalAction));
}

#[test]
fn two_blocked_by_queen_protection_on_king() {
    let mut state = empty_state();
    state.turn = 0;
    state.players[0].hand.push(c("2C"));
    state.players[1].royals.push(RoyalStack {
        base: c("QH"),
        base_owner: 1,
        jokers: Vec::new(),
    });
    state.players[1].royals.push(RoyalStack {
        base: c("KH"),
        base_owner: 1,
        jokers: Vec::new(),
    });

    let legal = state.legal_actions(0);
    assert!(!legal.contains(&Action::PlayOneOff {
        card: c("2C"),
        target: OneOffTarget::Royal { card: c("KH") }
    }));

    let err = state
        .apply(
            0,
            Action::PlayOneOff {
                card: c("2C"),
                target: OneOffTarget::Royal { card: c("KH") },
            },
        )
        .unwrap_err();
    assert!(matches!(err, RuleError::IllegalAction));
}

#[test]
fn counter_not_allowed_outside_countering() {
    let mut state = empty_state();
    state.turn = 0;
    let err = state.apply(0, Action::CounterPass).unwrap_err();
    assert!(matches!(err, RuleError::IllegalAction));
}

#[test]
fn counter_wrong_seat_not_your_turn() {
    let mut state = empty_state();
    state.turn = 0;
    state.players[0].hand.push(c("3C"));

    state
        .apply(
            0,
            Action::PlayOneOff {
                card: c("3C"),
                target: OneOffTarget::None,
            },
        )
        .unwrap();
    let err = state.apply(2, Action::CounterPass).unwrap_err();
    assert!(matches!(err, RuleError::IllegalAction));
}

#[test]
fn nine_returns_jack_and_freezes() {
    let mut state = empty_state();
    state.turn = 0;
    state.players[0].hand.push(c("9C"));
    state.players[2].points.push(PointStack {
        base: c("5C"),
        base_owner: 1,
        jacks: vec![JackOnStack {
            card: c("JD"),
            owner: 2,
        }],
    });

    state
        .apply(
            0,
            Action::PlayOneOff {
                card: c("9C"),
                target: OneOffTarget::Jack { card: c("JD") },
            },
        )
        .unwrap();
    state.apply(1, Action::CounterPass).unwrap();
    state.apply(2, Action::CounterPass).unwrap();

    assert!(state.players[2].hand.contains(&c("JD")));
    assert!(state.players[2].frozen.iter().any(|f| f.card == c("JD")));
    assert!(state.players[1].points.iter().any(|p| p.base == c("5C")));
}

#[test]
fn nine_returns_joker_and_freezes() {
    let mut state = empty_state();
    state.turn = 0;
    state.players[0].hand.push(c("9D"));
    state.players[2].royals.push(RoyalStack {
        base: c("KH"),
        base_owner: 1,
        jokers: vec![JokerOnStack {
            card: Card::Joker(0),
            owner: 2,
        }],
    });

    state
        .apply(
            0,
            Action::PlayOneOff {
                card: c("9D"),
                target: OneOffTarget::Joker {
                    card: Card::Joker(0),
                },
            },
        )
        .unwrap();
    state.apply(1, Action::CounterPass).unwrap();
    state.apply(2, Action::CounterPass).unwrap();

    assert!(state.players[2].hand.contains(&Card::Joker(0)));
    assert!(
        state.players[2]
            .frozen
            .iter()
            .any(|f| f.card == Card::Joker(0))
    );
    assert!(state.players[1].royals.iter().any(|r| r.base == c("KH")));
}

#[test]
fn nine_returns_royal_and_freezes() {
    let mut state = empty_state();
    state.turn = 0;
    state.players[0].hand.push(c("9S"));
    state.players[1].royals.push(RoyalStack {
        base: c("QH"),
        base_owner: 1,
        jokers: Vec::new(),
    });

    state
        .apply(
            0,
            Action::PlayOneOff {
                card: c("9S"),
                target: OneOffTarget::Royal { card: c("QH") },
            },
        )
        .unwrap();
    state.apply(1, Action::CounterPass).unwrap();
    state.apply(2, Action::CounterPass).unwrap();

    assert!(state.players[1].hand.contains(&c("QH")));
    assert!(state.players[1].frozen.iter().any(|f| f.card == c("QH")));
    assert!(state.players[1].royals.is_empty());
}

#[test]
fn public_view_with_glasses_reveals_hands() {
    let mut state = empty_state();
    state.turn = 0;
    state.players[0].royals.push(RoyalStack {
        base: c("8C"),
        base_owner: 0,
        jokers: Vec::new(),
    });
    state.players[0].hand = vec![c("9C")];
    state.players[1].hand = vec![c("AC"), c("KD")];
    state.players[2].hand = vec![c("2H")];

    let view0 = state.public_view(0);
    assert!(
        view0.players[1]
            .hand
            .iter()
            .all(|card| matches!(card, PublicCard::Known(_)))
    );
    assert!(
        view0.players[2]
            .hand
            .iter()
            .all(|card| matches!(card, PublicCard::Known(_)))
    );

    let view1 = state.public_view(1);
    assert!(matches!(view1.players[0].hand[0], PublicCard::Hidden));
}

#[test]
fn public_view_preserves_royal_stacks_across_viewers() {
    let mut state = empty_state();
    state.turn = 0;
    state.players[0].royals.push(RoyalStack {
        base: c("KH"),
        base_owner: 0,
        jokers: vec![JokerOnStack {
            card: Card::Joker(0),
            owner: 2,
        }],
    });
    state.players[1].royals.push(RoyalStack {
        base: c("8C"),
        base_owner: 1,
        jokers: Vec::new(),
    });
    state.players[2].royals.push(RoyalStack {
        base: c("QD"),
        base_owner: 2,
        jokers: Vec::new(),
    });

    let view0 = state.public_view(0);
    let view1 = state.public_view(1);
    let view2 = state.public_view(2);

    assert_eq!(view0.players[0].royals, view1.players[0].royals);
    assert_eq!(view1.players[0].royals, view2.players[0].royals);
    assert_eq!(view0.players[1].royals, view1.players[1].royals);
    assert_eq!(view1.players[1].royals, view2.players[1].royals);
    assert_eq!(view0.players[2].royals, view1.players[2].royals);
    assert_eq!(view1.players[2].royals, view2.players[2].royals);
    assert!(
        view0.players[1]
            .royals
            .iter()
            .any(|stack| stack.base == "8C")
    );
}

#[test]
fn three_cannot_pick_three_from_scrap() {
    let mut state = empty_state();
    state.turn = 0;
    state.players[0].hand.push(c("3C"));

    state
        .apply(
            0,
            Action::PlayOneOff {
                card: c("3C"),
                target: OneOffTarget::None,
            },
        )
        .unwrap();
    state.apply(1, Action::CounterPass).unwrap();
    state.apply(2, Action::CounterPass).unwrap();

    assert!(matches!(state.phase, Phase::Main));
    assert_eq!(state.turn, 1);
    assert!(!state.players[0].hand.contains(&c("3C")));
    assert!(state.scrap.contains(&c("3C")));
}

#[test]
fn four_target_empty_hand_resolves_immediately() {
    let mut state = empty_state();
    state.turn = 0;
    state.players[0].hand.push(c("4C"));

    state
        .apply(
            0,
            Action::PlayOneOff {
                card: c("4C"),
                target: OneOffTarget::Player { seat: 1 },
            },
        )
        .unwrap();
    state.apply(1, Action::CounterPass).unwrap();
    state.apply(2, Action::CounterPass).unwrap();

    assert!(matches!(state.phase, Phase::Main));
    assert_eq!(state.turn, 1);
}

#[test]
fn four_target_one_card_discard_finishes() {
    let mut state = empty_state();
    state.turn = 0;
    state.players[0].hand.push(c("4D"));
    state.players[1].hand.push(c("AC"));

    state
        .apply(
            0,
            Action::PlayOneOff {
                card: c("4D"),
                target: OneOffTarget::Player { seat: 1 },
            },
        )
        .unwrap();
    state.apply(1, Action::CounterPass).unwrap();
    state.apply(2, Action::CounterPass).unwrap();

    assert!(matches!(state.phase, Phase::ResolvingFour { seat: 1, .. }));
    state
        .apply(1, Action::ResolveFourDiscard { card: c("AC") })
        .unwrap();

    assert!(matches!(state.phase, Phase::Main));
    assert_eq!(state.turn, 1);
    assert!(state.players[1].hand.is_empty());
}

#[test]
fn five_with_empty_hand_auto_draws() {
    let mut state = empty_state();
    state.turn = 0;
    state.players[0].hand.push(c("5C"));
    state.deck = vec![c("AC"), c("KC"), c("QC")];

    state
        .apply(
            0,
            Action::PlayOneOff {
                card: c("5C"),
                target: OneOffTarget::None,
            },
        )
        .unwrap();
    state.apply(1, Action::CounterPass).unwrap();
    state.apply(2, Action::CounterPass).unwrap();

    assert!(matches!(state.phase, Phase::Main));
    assert_eq!(state.turn, 1);
    assert!(state.players[0].hand.contains(&c("AC")));
    assert!(state.players[0].hand.contains(&c("KC")));
    assert!(state.players[0].hand.contains(&c("QC")));
    assert!(state.deck.is_empty());
}

#[test]
fn seven_with_empty_deck_no_resolution() {
    let mut state = empty_state();
    state.turn = 0;
    state.players[0].hand.push(c("7C"));
    state.deck.clear();

    state
        .apply(
            0,
            Action::PlayOneOff {
                card: c("7C"),
                target: OneOffTarget::None,
            },
        )
        .unwrap();
    state.apply(1, Action::CounterPass).unwrap();
    state.apply(2, Action::CounterPass).unwrap();

    assert!(matches!(state.phase, Phase::Main));
    assert_eq!(state.turn, 1);
    assert!(state.scrap.contains(&c("7C")));
}

#[test]
fn seven_resolve_royal_from_reveal() {
    let mut state = empty_state();
    state.turn = 0;
    state.players[0].hand.push(c("7D"));
    state.deck = vec![c("KC"), c("2C")];

    state
        .apply(
            0,
            Action::PlayOneOff {
                card: c("7D"),
                target: OneOffTarget::None,
            },
        )
        .unwrap();
    state.apply(1, Action::CounterPass).unwrap();
    state.apply(2, Action::CounterPass).unwrap();
    state
        .apply(
            0,
            Action::ResolveSevenChoose {
                card: c("KC"),
                play: SevenPlay::Royal,
            },
        )
        .unwrap();

    assert!(state.players[0].royals.iter().any(|r| r.base == c("KC")));
    assert_eq!(state.deck, vec![c("2C")]);
    assert!(matches!(state.phase, Phase::Main));
    assert_eq!(state.turn, 1);
}

#[test]
fn scuttle_illegal_lower_suit_same_rank() {
    let mut state = empty_state();
    state.turn = 0;
    state.players[0].hand.push(c("9C"));
    state.players[1].points.push(PointStack {
        base: c("9D"),
        base_owner: 1,
        jacks: Vec::new(),
    });

    let err = state
        .apply(
            0,
            Action::Scuttle {
                card: c("9C"),
                target_point_base: c("9D"),
            },
        )
        .unwrap_err();
    assert!(matches!(err, RuleError::IllegalAction));
}

#[test]
fn draw_illegal_when_hand_full() {
    let mut state = empty_state();
    state.turn = 0;
    state.players[0].hand = vec![
        c("AC"),
        c("KC"),
        c("QC"),
        c("JC"),
        c("TC"),
        c("9C"),
        c("8C"),
    ];
    state.deck = vec![c("7C")];

    let err = state.apply(0, Action::Draw).unwrap_err();
    assert!(matches!(err, RuleError::IllegalAction));
}

#[test]
fn pass_illegal_when_deck_not_empty() {
    let mut state = empty_state();
    state.turn = 0;
    state.deck = vec![c("AC")];

    let err = state.apply(0, Action::Pass).unwrap_err();
    assert!(matches!(err, RuleError::IllegalAction));
}

#[test]
fn play_points_eight_is_legal() {
    let mut state = empty_state();
    state.turn = 0;
    state.players[0].hand.push(c("8C"));

    let legal = state.legal_actions(0);
    assert!(legal.contains(&Action::PlayPoints { card: c("8C") }));
    assert!(legal.contains(&Action::PlayRoyal { card: c("8C") }));

    state
        .apply(0, Action::PlayPoints { card: c("8C") })
        .unwrap();

    assert!(state.players[0].hand.is_empty());
    assert_eq!(state.players[0].points.len(), 1);
    assert_eq!(state.players[0].points[0].base, c("8C"));
    assert!(state.players[0].royals.is_empty());
}

#[test]
fn play_points_invalid_card() {
    let mut state = empty_state();
    state.turn = 0;
    state.players[0].hand.push(c("QH"));

    let err = state
        .apply(0, Action::PlayPoints { card: c("QH") })
        .unwrap_err();
    assert!(matches!(err, RuleError::IllegalAction));
}

#[test]
fn play_royal_invalid_card() {
    let mut state = empty_state();
    state.turn = 0;
    state.players[0].hand.push(c("9H"));

    let err = state
        .apply(0, Action::PlayRoyal { card: c("9H") })
        .unwrap_err();
    assert!(matches!(err, RuleError::IllegalAction));
}

#[test]
fn play_oneoff_invalid_card() {
    let mut state = empty_state();
    state.turn = 0;
    state.players[0].hand.push(c("KH"));

    let err = state
        .apply(
            0,
            Action::PlayOneOff {
                card: c("KH"),
                target: OneOffTarget::None,
            },
        )
        .unwrap_err();
    assert!(matches!(err, RuleError::IllegalAction));
}

#[test]
fn queen_protection_blocks_all_targets_with_two_queens() {
    let mut state = empty_state();
    state.turn = 0;
    state.players[0].hand.push(c("2C"));
    state.players[0].hand.push(c("9C"));
    state.players[0].hand.push(Card::Joker(0));
    state.players[1].royals.push(RoyalStack {
        base: c("QH"),
        base_owner: 1,
        jokers: Vec::new(),
    });
    state.players[1].royals.push(RoyalStack {
        base: c("QS"),
        base_owner: 1,
        jokers: Vec::new(),
    });

    let legal = state.legal_actions(0);
    assert!(!legal.contains(&Action::PlayOneOff {
        card: c("2C"),
        target: OneOffTarget::Royal { card: c("QH") }
    }));
    assert!(!legal.contains(&Action::PlayOneOff {
        card: c("9C"),
        target: OneOffTarget::Royal { card: c("QH") }
    }));
    assert!(!legal.contains(&Action::PlayJoker {
        joker: Card::Joker(0),
        target_royal_card: c("QH")
    }));
}

#[test]
fn queen_protection_allows_targeting_single_queen() {
    let mut state = empty_state();
    state.turn = 0;
    state.players[0].hand.push(c("2C"));
    state.players[0].hand.push(c("9C"));
    state.players[1].royals.push(RoyalStack {
        base: c("QH"),
        base_owner: 1,
        jokers: Vec::new(),
    });
    state.players[1].royals.push(RoyalStack {
        base: c("KH"),
        base_owner: 1,
        jokers: Vec::new(),
    });

    let legal = state.legal_actions(0);
    assert!(legal.contains(&Action::PlayOneOff {
        card: c("2C"),
        target: OneOffTarget::Royal { card: c("QH") }
    }));
    assert!(legal.contains(&Action::PlayOneOff {
        card: c("9C"),
        target: OneOffTarget::Royal { card: c("QH") }
    }));
    assert!(!legal.contains(&Action::PlayOneOff {
        card: c("2C"),
        target: OneOffTarget::Royal { card: c("KH") }
    }));
}

#[test]
fn pass_streak_resets_on_non_pass() {
    let mut state = empty_state();
    state.turn = 0;
    state.deck.clear();
    state.players[2].hand.push(c("5C"));

    state.apply(0, Action::Pass).unwrap();
    state.apply(1, Action::Pass).unwrap();
    state
        .apply(2, Action::PlayPoints { card: c("5C") })
        .unwrap();

    state.apply(0, Action::Pass).unwrap();
    state.apply(1, Action::Pass).unwrap();
    state.apply(2, Action::Pass).unwrap();
    assert!(state.winner.is_none());
    state.apply(0, Action::Pass).unwrap();
    assert_eq!(state.winner, Some(Winner::Draw));
}

#[test]
fn two_targets_royal_without_queen_protection() {
    let mut state = empty_state();
    state.turn = 0;
    state.players[0].hand.push(c("2C"));
    state.players[1].royals.push(RoyalStack {
        base: c("KH"),
        base_owner: 1,
        jokers: Vec::new(),
    });

    state
        .apply(
            0,
            Action::PlayOneOff {
                card: c("2C"),
                target: OneOffTarget::Royal { card: c("KH") },
            },
        )
        .unwrap();
    state.apply(1, Action::CounterPass).unwrap();
    state.apply(2, Action::CounterPass).unwrap();

    assert!(state.players[1].royals.is_empty());
    assert!(state.scrap.contains(&c("KH")));
}

#[test]
fn nine_oneoff_does_not_include_self_controlled_targets() {
    let mut state = empty_state();
    state.turn = 0;
    state.players[0].hand.push(c("9C"));

    state.players[0].points.push(PointStack {
        base: c("5C"),
        base_owner: 0,
        jacks: vec![JackOnStack {
            card: c("JD"),
            owner: 0,
        }],
    });
    state.players[1].points.push(PointStack {
        base: c("6D"),
        base_owner: 1,
        jacks: vec![JackOnStack {
            card: c("JH"),
            owner: 1,
        }],
    });

    state.players[0].royals.push(RoyalStack {
        base: c("KC"),
        base_owner: 0,
        jokers: vec![JokerOnStack {
            card: Card::Joker(0),
            owner: 0,
        }],
    });
    state.players[1].royals.push(RoyalStack {
        base: c("KH"),
        base_owner: 1,
        jokers: vec![JokerOnStack {
            card: Card::Joker(1),
            owner: 1,
        }],
    });

    let legal = state.legal_actions(0);
    assert!(!legal.contains(&Action::PlayOneOff {
        card: c("9C"),
        target: OneOffTarget::Point { base: c("5C") },
    }));
    assert!(!legal.contains(&Action::PlayOneOff {
        card: c("9C"),
        target: OneOffTarget::Royal { card: c("KC") },
    }));
    assert!(!legal.contains(&Action::PlayOneOff {
        card: c("9C"),
        target: OneOffTarget::Jack { card: c("JD") },
    }));
    assert!(!legal.contains(&Action::PlayOneOff {
        card: c("9C"),
        target: OneOffTarget::Joker {
            card: Card::Joker(0),
        },
    }));

    assert!(legal.contains(&Action::PlayOneOff {
        card: c("9C"),
        target: OneOffTarget::Point { base: c("6D") },
    }));
    assert!(legal.contains(&Action::PlayOneOff {
        card: c("9C"),
        target: OneOffTarget::Royal { card: c("KH") },
    }));
    assert!(legal.contains(&Action::PlayOneOff {
        card: c("9C"),
        target: OneOffTarget::Jack { card: c("JH") },
    }));
    assert!(legal.contains(&Action::PlayOneOff {
        card: c("9C"),
        target: OneOffTarget::Joker {
            card: Card::Joker(1),
        },
    }));
}

#[test]
fn two_oneoff_does_not_include_self_controlled_targets() {
    let mut state = empty_state();
    state.turn = 0;
    state.players[0].hand.push(c("2C"));

    state.players[0].points.push(PointStack {
        base: c("5C"),
        base_owner: 0,
        jacks: vec![JackOnStack {
            card: c("JD"),
            owner: 0,
        }],
    });
    state.players[1].points.push(PointStack {
        base: c("6D"),
        base_owner: 1,
        jacks: vec![JackOnStack {
            card: c("JH"),
            owner: 1,
        }],
    });

    state.players[0].royals.push(RoyalStack {
        base: c("KC"),
        base_owner: 0,
        jokers: vec![JokerOnStack {
            card: Card::Joker(0),
            owner: 0,
        }],
    });
    state.players[1].royals.push(RoyalStack {
        base: c("KH"),
        base_owner: 1,
        jokers: vec![JokerOnStack {
            card: Card::Joker(1),
            owner: 1,
        }],
    });

    let legal = state.legal_actions(0);
    assert!(!legal.contains(&Action::PlayOneOff {
        card: c("2C"),
        target: OneOffTarget::Royal { card: c("KC") },
    }));
    assert!(!legal.contains(&Action::PlayOneOff {
        card: c("2C"),
        target: OneOffTarget::Jack { card: c("JD") },
    }));
    assert!(!legal.contains(&Action::PlayOneOff {
        card: c("2C"),
        target: OneOffTarget::Joker {
            card: Card::Joker(0),
        },
    }));

    assert!(legal.contains(&Action::PlayOneOff {
        card: c("2C"),
        target: OneOffTarget::Royal { card: c("KH") },
    }));
    assert!(legal.contains(&Action::PlayOneOff {
        card: c("2C"),
        target: OneOffTarget::Jack { card: c("JH") },
    }));
    assert!(legal.contains(&Action::PlayOneOff {
        card: c("2C"),
        target: OneOffTarget::Joker {
            card: Card::Joker(1),
        },
    }));
}

#[test]
fn scuttle_uses_storage_seat_for_stack_removal() {
    let mut state = empty_state();
    state.turn = 0;
    state.players[0].hand.push(c("9C"));
    state.players[1].points.push(PointStack {
        base: c("5C"),
        base_owner: 1,
        jacks: vec![JackOnStack {
            card: c("JD"),
            owner: 2,
        }],
    });

    state
        .apply(
            0,
            Action::Scuttle {
                card: c("9C"),
                target_point_base: c("5C"),
            },
        )
        .unwrap();

    assert!(state.players[1].points.is_empty());
    assert!(state.scrap.contains(&c("9C")));
    assert!(state.scrap.contains(&c("5C")));
    assert!(state.scrap.contains(&c("JD")));
}

#[test]
fn play_joker_uses_storage_seat_for_stack_removal() {
    let mut state = empty_state();
    state.turn = 0;
    state.players[0].hand.push(Card::Joker(0));
    state.players[1].royals.push(RoyalStack {
        base: c("KH"),
        base_owner: 1,
        jokers: vec![JokerOnStack {
            card: Card::Joker(1),
            owner: 2,
        }],
    });

    state
        .apply(
            0,
            Action::PlayJoker {
                joker: Card::Joker(0),
                target_royal_card: c("KH"),
            },
        )
        .unwrap();

    assert!(state.players[1].royals.is_empty());
    assert_eq!(state.players[0].royals.len(), 1);
    assert_eq!(state.players[0].royals[0].base, c("KH"));
    assert_eq!(state.players[0].royals[0].jokers.len(), 2);
}
