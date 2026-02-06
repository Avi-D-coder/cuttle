use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::action::{Action, OneOffTarget, SevenPlay};
use crate::card::{Card, Rank, full_deck_with_jokers};

pub type Seat = u8;

pub const PLAYER_COUNT: u8 = 3;
pub const HAND_LIMIT: usize = 7;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Winner {
    Seat(Seat),
    Draw,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Phase {
    Main,
    Countering(CounterState),
    ResolvingThree {
        seat: Seat,
        base_player: Seat,
    },
    ResolvingFour {
        seat: Seat,
        base_player: Seat,
        remaining: u8,
    },
    ResolvingFive {
        seat: Seat,
        base_player: Seat,
        discarded: bool,
    },
    ResolvingSeven {
        seat: Seat,
        base_player: Seat,
        revealed: Vec<Card>,
    },
    GameOver,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CounterState {
    pub base_player: Seat,
    pub oneoff: Action,
    pub twos: Vec<(Seat, Card)>,
    pub next_seat: Seat,
    pub rotation_anchor: Seat,
}

#[derive(Clone, Debug)]
pub struct FrozenCard {
    pub card: Card,
    pub remaining_turns: u8,
}

#[derive(Clone, Debug)]
pub struct JackOnStack {
    pub card: Card,
    pub owner: Seat,
}

#[derive(Clone, Debug)]
pub struct JokerOnStack {
    pub card: Card,
    pub owner: Seat,
}

#[derive(Clone, Debug)]
pub struct PointStack {
    pub base: Card,
    pub base_owner: Seat,
    pub jacks: Vec<JackOnStack>,
}

impl PointStack {
    pub fn controller(&self) -> Seat {
        self.jacks
            .last()
            .map(|j| j.owner)
            .unwrap_or(self.base_owner)
    }
}

#[derive(Clone, Debug)]
pub struct RoyalStack {
    pub base: Card,
    pub base_owner: Seat,
    pub jokers: Vec<JokerOnStack>,
}

impl RoyalStack {
    pub fn controller(&self) -> Seat {
        self.jokers
            .last()
            .map(|j| j.owner)
            .unwrap_or(self.base_owner)
    }
}

#[derive(Clone, Debug)]
pub struct PlayerState {
    pub hand: Vec<Card>,
    pub points: Vec<PointStack>,
    pub royals: Vec<RoyalStack>,
    pub frozen: Vec<FrozenCard>,
}

impl PlayerState {
    pub fn new() -> Self {
        Self {
            hand: Vec::new(),
            points: Vec::new(),
            royals: Vec::new(),
            frozen: Vec::new(),
        }
    }
}

impl Default for PlayerState {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug)]
pub struct CutthroatState {
    pub dealer: Seat,
    pub turn: Seat,
    pub phase: Phase,
    pub deck: Vec<Card>,
    pub scrap: Vec<Card>,
    pub players: Vec<PlayerState>,
    pub pass_streak_start: Option<Seat>,
    pub pass_streak_len: u8,
    pub winner: Option<Winner>,
}

impl CutthroatState {
    pub fn new_with_deck(dealer: Seat, mut deck: Vec<Card>) -> Self {
        let mut players = vec![PlayerState::new(), PlayerState::new(), PlayerState::new()];
        let mut current = next_seat(dealer);
        for _ in 0..5 {
            for _ in 0..PLAYER_COUNT {
                let card = deck.remove(0);
                players[current as usize].hand.push(card);
                current = next_seat(current);
            }
        }

        let turn = next_seat(dealer);
        Self {
            dealer,
            turn,
            phase: Phase::Main,
            deck,
            scrap: Vec::new(),
            players,
            pass_streak_start: None,
            pass_streak_len: 0,
            winner: None,
        }
    }

    pub fn new_shuffled(dealer: Seat) -> Self {
        let mut deck = full_deck_with_jokers();
        let mut rng = rand::thread_rng();
        deck.shuffle(&mut rng);
        Self::new_with_deck(dealer, deck)
    }

    pub fn legal_actions(&self, seat: Seat) -> Vec<Action> {
        if matches!(self.phase, Phase::GameOver) {
            return vec![];
        }

        match &self.phase {
            Phase::Main => self.legal_main_actions(seat),
            Phase::Countering(counter) => {
                if seat != counter.next_seat {
                    return vec![];
                }
                self.legal_counter_actions(seat)
            }
            Phase::ResolvingThree { seat: s, .. } => {
                if seat != *s {
                    return vec![];
                }
                self.scrap
                    .iter()
                    .map(|card| Action::ResolveThreePick {
                        card_from_scrap: *card,
                    })
                    .collect()
            }
            Phase::ResolvingFour { seat: s, .. } => {
                if seat != *s {
                    return vec![];
                }
                self.players[seat as usize]
                    .hand
                    .iter()
                    .map(|card| Action::ResolveFourDiscard { card: *card })
                    .collect()
            }
            Phase::ResolvingFive {
                seat: s, discarded, ..
            } => {
                if seat != *s || *discarded {
                    return vec![];
                }
                self.players[seat as usize]
                    .hand
                    .iter()
                    .map(|card| Action::ResolveFiveDiscard { card: *card })
                    .collect()
            }
            Phase::ResolvingSeven {
                seat: s, revealed, ..
            } => {
                if seat != *s {
                    return vec![];
                }
                self.legal_seven_actions(*s, revealed)
            }
            Phase::GameOver => vec![],
        }
    }

    pub fn apply(&mut self, seat: Seat, action: Action) -> Result<Vec<Event>, RuleError> {
        if matches!(self.phase, Phase::GameOver) {
            return Err(RuleError::GameOver);
        }
        let legal = self.legal_actions(seat);
        if !legal.contains(&action) {
            return Err(RuleError::IllegalAction);
        }

        let phase = self.phase.clone();
        match phase {
            Phase::Main => self.apply_main_action(seat, action)?,
            Phase::Countering(_) => self.apply_counter_action(seat, action)?,
            Phase::ResolvingThree { seat: s, .. } => {
                if seat != s {
                    return Err(RuleError::NotYourTurn);
                }
                self.apply_resolve_three(seat, action)?;
            }
            Phase::ResolvingFour { seat: s, .. } => {
                if seat != s {
                    return Err(RuleError::NotYourTurn);
                }
                self.apply_resolve_four(seat, action)?;
            }
            Phase::ResolvingFive { seat: s, .. } => {
                if seat != s {
                    return Err(RuleError::NotYourTurn);
                }
                self.apply_resolve_five(seat, action)?;
            }
            Phase::ResolvingSeven { seat: s, .. } => {
                if seat != s {
                    return Err(RuleError::NotYourTurn);
                }
                self.apply_resolve_seven(seat, action)?;
            }
            Phase::GameOver => {}
        }

        let mut events = Vec::new();
        if let Some(winner) = self.check_winner() {
            self.winner = Some(winner.clone());
            self.phase = Phase::GameOver;
            events.push(Event::GameOver(winner));
        }
        Ok(events)
    }

    pub fn public_view(&self, viewer: Seat) -> PublicView {
        let viewer_has_glasses = self.player_has_glasses(viewer);
        let players = self
            .players
            .iter()
            .enumerate()
            .map(|(idx, player)| {
                let seat = idx as Seat;
                let show_hand = seat == viewer || viewer_has_glasses;
                let hand = if show_hand {
                    player
                        .hand
                        .iter()
                        .map(|c| PublicCard::Known(c.to_token()))
                        .collect()
                } else {
                    player.hand.iter().map(|_| PublicCard::Hidden).collect()
                };

                let points = player
                    .points
                    .iter()
                    .map(|stack| PointStackView {
                        base: stack.base.to_token(),
                        controller: stack.controller(),
                        jacks: stack.jacks.iter().map(|j| j.card.to_token()).collect(),
                    })
                    .collect();
                let royals = player
                    .royals
                    .iter()
                    .map(|stack| RoyalStackView {
                        base: stack.base.to_token(),
                        controller: stack.controller(),
                        jokers: stack.jokers.iter().map(|j| j.card.to_token()).collect(),
                    })
                    .collect();

                let frozen = if seat == viewer {
                    player.frozen.iter().map(|f| f.card.to_token()).collect()
                } else {
                    Vec::new()
                };

                PlayerView {
                    seat,
                    hand,
                    points,
                    royals,
                    frozen,
                }
            })
            .collect();

        PublicView {
            seat: viewer,
            turn: self.turn,
            phase: self.phase.view(viewer),
            deck_count: self.deck.len(),
            scrap: self.scrap.iter().map(|c| c.to_token()).collect(),
            players,
            last_event: None,
        }
    }

    fn legal_main_actions(&self, seat: Seat) -> Vec<Action> {
        if seat != self.turn {
            return vec![];
        }
        let player = &self.players[seat as usize];
        let available = self.available_cards(seat);
        let mut actions = Vec::new();

        if !self.deck.is_empty() && player.hand.len() < HAND_LIMIT {
            actions.push(Action::Draw);
        }
        if self.deck.is_empty() {
            actions.push(Action::Pass);
        }

        for &card in &available {
            if is_point_card(card) {
                actions.push(Action::PlayPoints { card });
            }
        }

        for &card in &available {
            if is_point_card(card) {
                for (owner, stack) in self.iter_point_targets() {
                    if owner == seat {
                        continue;
                    }
                    if card.scuttle_beats(stack.base) {
                        actions.push(Action::Scuttle {
                            card,
                            target_point_base: stack.base,
                        });
                    }
                }
            }
        }

        for &card in &available {
            if matches!(
                card,
                Card::Standard {
                    rank: Rank::Eight | Rank::Queen | Rank::King,
                    ..
                }
            ) {
                actions.push(Action::PlayRoyal { card });
            }
        }

        for &card in &available {
            if matches!(
                card,
                Card::Standard {
                    rank: Rank::Jack,
                    ..
                }
            ) {
                for (owner, stack) in self.iter_point_targets() {
                    if owner == seat {
                        continue;
                    }
                    if self.can_target(owner, TargetKind::Point, stack.base) {
                        actions.push(Action::PlayJack {
                            jack: card,
                            target_point_base: stack.base,
                        });
                    }
                }
            }
        }

        for &card in &available {
            if matches!(card, Card::Joker(_)) {
                for (owner, stack) in self.iter_royal_targets() {
                    if owner == seat {
                        continue;
                    }
                    if self.can_target(owner, TargetKind::Royal, stack.base) {
                        actions.push(Action::PlayJoker {
                            joker: card,
                            target_royal_card: stack.base,
                        });
                    }
                }
            }
        }

        for &card in &available {
            if card.is_oneoff() {
                actions.extend(self.oneoff_actions_for_card(seat, card));
            }
        }

        actions
    }

    fn legal_counter_actions(&self, seat: Seat) -> Vec<Action> {
        let mut actions = vec![Action::CounterPass];
        let available = self.available_cards(seat);
        for &card in &available {
            if matches!(
                card,
                Card::Standard {
                    rank: Rank::Two,
                    ..
                }
            ) {
                actions.push(Action::CounterTwo { two_card: card });
            }
        }
        actions
    }

    fn legal_seven_actions(&self, seat: Seat, revealed: &[Card]) -> Vec<Action> {
        let mut actions = Vec::new();
        for (idx, card) in revealed.iter().enumerate() {
            let idx = idx as u8;
            let mut plays = Vec::new();
            match card {
                Card::Standard { rank, .. } => match rank {
                    Rank::Jack => {
                        for (owner, stack) in self.iter_point_targets() {
                            if owner == seat {
                                continue;
                            }
                            if self.can_target(owner, TargetKind::Point, stack.base) {
                                plays.push(SevenPlay::Jack { target: stack.base });
                            }
                        }
                    }
                    Rank::Eight | Rank::Queen | Rank::King => {
                        plays.push(SevenPlay::Royal);
                    }
                    Rank::Ace
                    | Rank::Two
                    | Rank::Three
                    | Rank::Four
                    | Rank::Five
                    | Rank::Six
                    | Rank::Seven
                    | Rank::Nine => {
                        for action in self.oneoff_actions_for_card(seat, *card) {
                            if let Action::PlayOneOff { target, .. } = action {
                                plays.push(SevenPlay::OneOff { target });
                            }
                        }
                    }
                    _ => {}
                },
                Card::Joker(_) => {
                    for (owner, stack) in self.iter_royal_targets() {
                        if owner == seat {
                            continue;
                        }
                        if self.can_target(owner, TargetKind::Royal, stack.base) {
                            plays.push(SevenPlay::Joker { target: stack.base });
                        }
                    }
                }
            }

            if is_point_card(*card) {
                plays.push(SevenPlay::Points);
                for (owner, stack) in self.iter_point_targets() {
                    if owner == seat {
                        continue;
                    }
                    if card.scuttle_beats(stack.base) {
                        plays.push(SevenPlay::Scuttle { target: stack.base });
                    }
                }
            }

            if plays.is_empty() {
                actions.push(Action::ResolveSevenChoose {
                    source_index: idx,
                    play: SevenPlay::Discard,
                });
            } else {
                for play in plays {
                    actions.push(Action::ResolveSevenChoose {
                        source_index: idx,
                        play,
                    });
                }
            }
        }
        actions
    }

    fn oneoff_actions_for_card(&self, seat: Seat, card: Card) -> Vec<Action> {
        let mut actions = Vec::new();
        if let Card::Standard { rank, .. } = card {
            match rank {
                Rank::Ace | Rank::Three | Rank::Five | Rank::Six | Rank::Seven => {
                    actions.push(Action::PlayOneOff {
                        card,
                        target: OneOffTarget::None,
                    });
                }
                Rank::Two => {
                    for (owner, stack) in self.iter_royal_targets() {
                        if self.can_target(owner, TargetKind::Royal, stack.base) {
                            actions.push(Action::PlayOneOff {
                                card,
                                target: OneOffTarget::Royal { card: stack.base },
                            });
                        }
                    }
                    for (owner, _stack, jack) in self.iter_jack_targets() {
                        if self.can_target(owner, TargetKind::Jack, jack) {
                            actions.push(Action::PlayOneOff {
                                card,
                                target: OneOffTarget::Jack { card: jack },
                            });
                        }
                    }
                    for (owner, _stack, joker) in self.iter_joker_targets() {
                        if self.can_target(owner, TargetKind::Joker, joker) {
                            actions.push(Action::PlayOneOff {
                                card,
                                target: OneOffTarget::Joker { card: joker },
                            });
                        }
                    }
                }
                Rank::Four => {
                    for target in 0..PLAYER_COUNT {
                        if target != seat {
                            actions.push(Action::PlayOneOff {
                                card,
                                target: OneOffTarget::Player { seat: target },
                            });
                        }
                    }
                }
                Rank::Nine => {
                    for (owner, stack) in self.iter_point_targets() {
                        if self.can_target(owner, TargetKind::Point, stack.base) {
                            actions.push(Action::PlayOneOff {
                                card,
                                target: OneOffTarget::Point { base: stack.base },
                            });
                        }
                    }
                    for (owner, stack) in self.iter_royal_targets() {
                        if self.can_target(owner, TargetKind::Royal, stack.base) {
                            actions.push(Action::PlayOneOff {
                                card,
                                target: OneOffTarget::Royal { card: stack.base },
                            });
                        }
                    }
                    for (owner, _stack, jack) in self.iter_jack_targets() {
                        if self.can_target(owner, TargetKind::Jack, jack) {
                            actions.push(Action::PlayOneOff {
                                card,
                                target: OneOffTarget::Jack { card: jack },
                            });
                        }
                    }
                    for (owner, _stack, joker) in self.iter_joker_targets() {
                        if self.can_target(owner, TargetKind::Joker, joker) {
                            actions.push(Action::PlayOneOff {
                                card,
                                target: OneOffTarget::Joker { card: joker },
                            });
                        }
                    }
                }
                _ => {}
            }
        }
        actions
    }

    fn apply_main_action(&mut self, seat: Seat, action: Action) -> Result<(), RuleError> {
        match action {
            Action::Draw => {
                if self.deck.is_empty() {
                    return Err(RuleError::InvalidAction);
                }
                if self.players[seat as usize].hand.len() >= HAND_LIMIT {
                    return Err(RuleError::InvalidAction);
                }
                let card = self.deck.remove(0);
                self.players[seat as usize].hand.push(card);
                self.reset_pass_streak();
                self.finish_turn(seat);
            }
            Action::Pass => {
                if !self.deck.is_empty() {
                    return Err(RuleError::InvalidAction);
                }
                self.advance_pass_streak(seat);
                if self.is_stalemate(seat) {
                    self.winner = Some(Winner::Draw);
                    self.phase = Phase::GameOver;
                    return Ok(());
                }
                self.finish_turn(seat);
            }
            Action::PlayPoints { card } => {
                if !is_point_card(card) {
                    return Err(RuleError::InvalidAction);
                }
                self.remove_from_hand(seat, card)?;
                self.players[seat as usize].points.push(PointStack {
                    base: card,
                    base_owner: seat,
                    jacks: Vec::new(),
                });
                self.reset_pass_streak();
                self.finish_turn(seat);
            }
            Action::Scuttle {
                card,
                target_point_base,
            } => {
                self.remove_from_hand(seat, card)?;
                let (stack_seat, idx) = self
                    .find_point_stack_by_base(target_point_base)
                    .ok_or(RuleError::InvalidAction)?;
                let target_owner = self.players[stack_seat as usize].points[idx].controller();
                if target_owner == seat {
                    return Err(RuleError::InvalidAction);
                }
                if !card.scuttle_beats(target_point_base) {
                    return Err(RuleError::InvalidAction);
                }
                let stack = self.players[stack_seat as usize].points.remove(idx);
                self.scrap.push(card);
                self.scrap.push(stack.base);
                for jack in stack.jacks {
                    self.scrap.push(jack.card);
                }
                self.reset_pass_streak();
                self.finish_turn(seat);
            }
            Action::PlayRoyal { card } => {
                if !matches!(
                    card,
                    Card::Standard {
                        rank: Rank::Eight | Rank::Queen | Rank::King,
                        ..
                    }
                ) {
                    return Err(RuleError::InvalidAction);
                }
                self.remove_from_hand(seat, card)?;
                self.players[seat as usize].royals.push(RoyalStack {
                    base: card,
                    base_owner: seat,
                    jokers: Vec::new(),
                });
                self.reset_pass_streak();
                self.finish_turn(seat);
            }
            Action::PlayJack {
                jack,
                target_point_base,
            } => {
                if !matches!(
                    jack,
                    Card::Standard {
                        rank: Rank::Jack,
                        ..
                    }
                ) {
                    return Err(RuleError::InvalidAction);
                }
                let (stack_seat, idx) = self
                    .find_point_stack_by_base(target_point_base)
                    .ok_or(RuleError::InvalidAction)?;
                let target_owner = self.players[stack_seat as usize].points[idx].controller();
                if target_owner == seat {
                    return Err(RuleError::InvalidAction);
                }
                if !self.can_target(target_owner, TargetKind::Point, target_point_base) {
                    return Err(RuleError::InvalidAction);
                }
                self.remove_from_hand(seat, jack)?;
                let mut stack = self.players[stack_seat as usize].points.remove(idx);
                stack.jacks.push(JackOnStack {
                    card: jack,
                    owner: seat,
                });
                self.players[seat as usize].points.push(stack);
                self.reset_pass_streak();
                self.finish_turn(seat);
            }
            Action::PlayJoker {
                joker,
                target_royal_card,
            } => {
                if !matches!(joker, Card::Joker(_)) {
                    return Err(RuleError::InvalidAction);
                }
                let (stack_seat, idx) = self
                    .find_royal_stack_by_base(target_royal_card)
                    .ok_or(RuleError::InvalidAction)?;
                let target_owner = self.players[stack_seat as usize].royals[idx].controller();
                if target_owner == seat {
                    return Err(RuleError::InvalidAction);
                }
                if !self.can_target(target_owner, TargetKind::Royal, target_royal_card) {
                    return Err(RuleError::InvalidAction);
                }
                self.remove_from_hand(seat, joker)?;
                let mut stack = self.players[stack_seat as usize].royals.remove(idx);
                stack.jokers.push(JokerOnStack {
                    card: joker,
                    owner: seat,
                });
                self.players[seat as usize].royals.push(stack);
                self.reset_pass_streak();
                self.finish_turn(seat);
            }
            Action::PlayOneOff { card, target } => {
                if !card.is_oneoff() {
                    return Err(RuleError::InvalidAction);
                }
                self.remove_from_hand(seat, card)?;
                self.reset_pass_streak();
                let counter = CounterState {
                    base_player: seat,
                    oneoff: Action::PlayOneOff { card, target },
                    twos: Vec::new(),
                    next_seat: next_seat(seat),
                    rotation_anchor: seat,
                };
                self.phase = Phase::Countering(counter);
            }
            _ => return Err(RuleError::InvalidAction),
        }
        Ok(())
    }

    fn apply_counter_action(&mut self, seat: Seat, action: Action) -> Result<(), RuleError> {
        let mut counter = match std::mem::replace(&mut self.phase, Phase::Main) {
            Phase::Countering(counter) => counter,
            other => {
                self.phase = other;
                return Err(RuleError::InvalidAction);
            }
        };
        if seat != counter.next_seat {
            self.phase = Phase::Countering(counter);
            return Err(RuleError::NotYourTurn);
        }
        match action {
            Action::CounterTwo { two_card } => {
                if !matches!(
                    two_card,
                    Card::Standard {
                        rank: Rank::Two,
                        ..
                    }
                ) {
                    self.phase = Phase::Countering(counter);
                    return Err(RuleError::InvalidAction);
                }
                if let Err(err) = self.remove_from_hand(seat, two_card) {
                    self.phase = Phase::Countering(counter);
                    return Err(err);
                }
                counter.twos.push((seat, two_card));
                counter.rotation_anchor = seat;
                counter.next_seat = next_seat(counter.next_seat);
            }
            Action::CounterPass => {
                counter.next_seat = next_seat(counter.next_seat);
            }
            _ => {
                self.phase = Phase::Countering(counter);
                return Err(RuleError::InvalidAction);
            }
        }

        let should_end =
            matches!(action, Action::CounterPass) && counter.next_seat == counter.rotation_anchor;
        if should_end {
            let base_player = counter.base_player;
            let oneoff = counter.oneoff.clone();
            let twos = std::mem::take(&mut counter.twos);
            let two_count = twos.len();
            self.scrap.extend(twos.into_iter().map(|(_, c)| c));
            self.phase = Phase::Main;
            let resolve = (two_count % 2) == 0;
            match oneoff {
                Action::PlayOneOff { card, target } => {
                    self.scrap.push(card);
                    if resolve {
                        self.resolve_oneoff(base_player, card, target)?;
                    }
                    if !matches!(
                        self.phase,
                        Phase::ResolvingThree { .. }
                            | Phase::ResolvingFour { .. }
                            | Phase::ResolvingFive { .. }
                            | Phase::ResolvingSeven { .. }
                    ) {
                        self.finish_turn(base_player);
                    }
                }
                _ => return Err(RuleError::InvalidAction),
            }
        } else {
            self.phase = Phase::Countering(counter);
        }
        Ok(())
    }

    fn resolve_oneoff(
        &mut self,
        base_player: Seat,
        card: Card,
        target: OneOffTarget,
    ) -> Result<(), RuleError> {
        match card {
            Card::Standard { rank, .. } => match rank {
                Rank::Ace => {
                    self.scrap_all_points();
                }
                Rank::Two => {
                    self.apply_two_target(target)?;
                }
                Rank::Three => {
                    if !self.scrap.is_empty() {
                        self.phase = Phase::ResolvingThree {
                            seat: base_player,
                            base_player,
                        };
                    }
                }
                Rank::Four => {
                    let OneOffTarget::Player { seat } = target else {
                        return Err(RuleError::InvalidAction);
                    };
                    if !self.players[seat as usize].hand.is_empty() {
                        self.phase = Phase::ResolvingFour {
                            seat,
                            base_player,
                            remaining: 2,
                        };
                    }
                }
                Rank::Five => {
                    if self.players[base_player as usize].hand.is_empty() {
                        let mut draws = 3;
                        while draws > 0
                            && !self.deck.is_empty()
                            && self.players[base_player as usize].hand.len() < HAND_LIMIT
                        {
                            let card = self.deck.remove(0);
                            self.players[base_player as usize].hand.push(card);
                            draws -= 1;
                        }
                    } else {
                        self.phase = Phase::ResolvingFive {
                            seat: base_player,
                            base_player,
                            discarded: false,
                        };
                    }
                }
                Rank::Six => {
                    self.scrap_all_royals();
                }
                Rank::Seven => {
                    let revealed = self.reveal_top_two();
                    if !revealed.is_empty() {
                        self.phase = Phase::ResolvingSeven {
                            seat: base_player,
                            base_player,
                            revealed,
                        };
                    }
                }
                Rank::Nine => {
                    self.apply_nine_target(target)?;
                }
                _ => {}
            },
            _ => return Err(RuleError::InvalidAction),
        }
        Ok(())
    }

    fn apply_resolve_three(&mut self, seat: Seat, action: Action) -> Result<(), RuleError> {
        let Action::ResolveThreePick { card_from_scrap } = action else {
            return Err(RuleError::InvalidAction);
        };
        let idx = self
            .scrap
            .iter()
            .position(|c| *c == card_from_scrap)
            .ok_or(RuleError::InvalidAction)?;
        let card = self.scrap.remove(idx);
        self.players[seat as usize].hand.push(card);
        let base_player = match self.phase {
            Phase::ResolvingThree { base_player, .. } => base_player,
            _ => seat,
        };
        self.phase = Phase::Main;
        self.finish_turn(base_player);
        Ok(())
    }

    fn apply_resolve_four(&mut self, seat: Seat, action: Action) -> Result<(), RuleError> {
        let Action::ResolveFourDiscard { card } = action else {
            return Err(RuleError::InvalidAction);
        };
        self.remove_from_hand(seat, card)?;
        self.scrap.push(card);
        let (base_player, remaining) = match self.phase {
            Phase::ResolvingFour {
                base_player,
                remaining,
                ..
            } => (base_player, remaining),
            _ => (seat, 0),
        };
        let remaining = remaining.saturating_sub(1);
        if remaining == 0 || self.players[seat as usize].hand.is_empty() {
            self.phase = Phase::Main;
            self.finish_turn(base_player);
        } else {
            self.phase = Phase::ResolvingFour {
                seat,
                base_player,
                remaining,
            };
        }
        Ok(())
    }

    fn apply_resolve_five(&mut self, seat: Seat, action: Action) -> Result<(), RuleError> {
        let Action::ResolveFiveDiscard { card } = action else {
            return Err(RuleError::InvalidAction);
        };
        self.remove_from_hand(seat, card)?;
        self.scrap.push(card);
        let base_player = match self.phase {
            Phase::ResolvingFive { base_player, .. } => base_player,
            _ => seat,
        };
        let mut draws = 3;
        while draws > 0
            && !self.deck.is_empty()
            && self.players[seat as usize].hand.len() < HAND_LIMIT
        {
            let card = self.deck.remove(0);
            self.players[seat as usize].hand.push(card);
            draws -= 1;
        }
        self.phase = Phase::Main;
        self.finish_turn(base_player);
        Ok(())
    }

    fn apply_resolve_seven(&mut self, seat: Seat, action: Action) -> Result<(), RuleError> {
        let Action::ResolveSevenChoose { source_index, play } = action else {
            return Err(RuleError::InvalidAction);
        };
        let (base_player, mut revealed) = match &self.phase {
            Phase::ResolvingSeven {
                base_player,
                revealed,
                ..
            } => (*base_player, revealed.clone()),
            _ => (seat, Vec::new()),
        };
        if source_index as usize >= revealed.len() {
            return Err(RuleError::InvalidAction);
        }
        let chosen = revealed.remove(source_index as usize);
        if let Some(unchosen) = revealed.pop() {
            self.deck.insert(0, unchosen);
        }

        match play {
            SevenPlay::Discard => {
                self.scrap.push(chosen);
            }
            SevenPlay::Points => {
                if !is_point_card(chosen) {
                    return Err(RuleError::InvalidAction);
                }
                self.players[seat as usize].points.push(PointStack {
                    base: chosen,
                    base_owner: seat,
                    jacks: Vec::new(),
                });
            }
            SevenPlay::Scuttle { target } => {
                if !is_point_card(chosen) {
                    return Err(RuleError::InvalidAction);
                }
                let (stack_seat, idx) = self
                    .find_point_stack_by_base(target)
                    .ok_or(RuleError::InvalidAction)?;
                let target_owner = self.players[stack_seat as usize].points[idx].controller();
                if target_owner == seat || !chosen.scuttle_beats(target) {
                    return Err(RuleError::InvalidAction);
                }
                let stack = self.players[stack_seat as usize].points.remove(idx);
                self.scrap.push(chosen);
                self.scrap.push(stack.base);
                for jack in stack.jacks {
                    self.scrap.push(jack.card);
                }
            }
            SevenPlay::Royal => {
                if !matches!(
                    chosen,
                    Card::Standard {
                        rank: Rank::Eight | Rank::Queen | Rank::King,
                        ..
                    }
                ) {
                    return Err(RuleError::InvalidAction);
                }
                self.players[seat as usize].royals.push(RoyalStack {
                    base: chosen,
                    base_owner: seat,
                    jokers: Vec::new(),
                });
            }
            SevenPlay::Joker { target } => {
                if !matches!(chosen, Card::Joker(_)) {
                    return Err(RuleError::InvalidAction);
                }
                let (stack_seat, idx) = self
                    .find_royal_stack_by_base(target)
                    .ok_or(RuleError::InvalidAction)?;
                let target_owner = self.players[stack_seat as usize].royals[idx].controller();
                if target_owner == seat || !self.can_target(target_owner, TargetKind::Royal, target)
                {
                    return Err(RuleError::InvalidAction);
                }
                let mut stack = self.players[stack_seat as usize].royals.remove(idx);
                stack.jokers.push(JokerOnStack {
                    card: chosen,
                    owner: seat,
                });
                self.players[seat as usize].royals.push(stack);
            }
            SevenPlay::Jack { target } => {
                if !matches!(
                    chosen,
                    Card::Standard {
                        rank: Rank::Jack,
                        ..
                    }
                ) {
                    return Err(RuleError::InvalidAction);
                }
                let (stack_seat, idx) = self
                    .find_point_stack_by_base(target)
                    .ok_or(RuleError::InvalidAction)?;
                let target_owner = self.players[stack_seat as usize].points[idx].controller();
                if target_owner == seat || !self.can_target(target_owner, TargetKind::Point, target)
                {
                    return Err(RuleError::InvalidAction);
                }
                let mut stack = self.players[stack_seat as usize].points.remove(idx);
                stack.jacks.push(JackOnStack {
                    card: chosen,
                    owner: seat,
                });
                self.players[seat as usize].points.push(stack);
            }
            SevenPlay::OneOff { target } => {
                if !chosen.is_oneoff() {
                    return Err(RuleError::InvalidAction);
                }
                let counter = CounterState {
                    base_player: seat,
                    oneoff: Action::PlayOneOff {
                        card: chosen,
                        target,
                    },
                    twos: Vec::new(),
                    next_seat: next_seat(seat),
                    rotation_anchor: seat,
                };
                self.phase = Phase::Countering(counter);
            }
        }

        if !matches!(
            self.phase,
            Phase::ResolvingThree { .. }
                | Phase::ResolvingFour { .. }
                | Phase::ResolvingFive { .. }
                | Phase::Countering(_)
        ) {
            self.phase = Phase::Main;
            self.finish_turn(base_player);
        }
        Ok(())
    }

    fn apply_two_target(&mut self, target: OneOffTarget) -> Result<(), RuleError> {
        match target {
            OneOffTarget::Royal { card } => {
                let (stack_seat, idx) = self
                    .find_royal_stack_by_base(card)
                    .ok_or(RuleError::InvalidAction)?;
                let target_owner = self.players[stack_seat as usize].royals[idx].controller();
                if !self.can_target(target_owner, TargetKind::Royal, card) {
                    return Err(RuleError::InvalidAction);
                }
                let stack = self.players[stack_seat as usize].royals.remove(idx);
                self.scrap.push(stack.base);
                for joker in stack.jokers {
                    self.scrap.push(joker.card);
                }
            }
            OneOffTarget::Jack { card } => {
                self.scrap_top_jack(card)?;
            }
            OneOffTarget::Joker { card } => {
                self.scrap_top_joker(card)?;
            }
            _ => return Err(RuleError::InvalidAction),
        }
        Ok(())
    }

    fn apply_nine_target(&mut self, target: OneOffTarget) -> Result<(), RuleError> {
        match target {
            OneOffTarget::Point { base } => {
                let (stack_seat, idx) = self
                    .find_point_stack_by_base(base)
                    .ok_or(RuleError::InvalidAction)?;
                let target_owner = self.players[stack_seat as usize].points[idx].controller();
                if !self.can_target(target_owner, TargetKind::Point, base) {
                    return Err(RuleError::InvalidAction);
                }
                let stack = self.players[stack_seat as usize].points.remove(idx);
                self.players[target_owner as usize].hand.push(stack.base);
                self.players[target_owner as usize].frozen.push(FrozenCard {
                    card: stack.base,
                    remaining_turns: 1,
                });
                for jack in stack.jacks {
                    self.scrap.push(jack.card);
                }
            }
            OneOffTarget::Royal { card } => {
                let (stack_seat, idx) = self
                    .find_royal_stack_by_base(card)
                    .ok_or(RuleError::InvalidAction)?;
                let target_owner = self.players[stack_seat as usize].royals[idx].controller();
                if !self.can_target(target_owner, TargetKind::Royal, card) {
                    return Err(RuleError::InvalidAction);
                }
                let stack = self.players[stack_seat as usize].royals.remove(idx);
                self.players[target_owner as usize].hand.push(stack.base);
                self.players[target_owner as usize].frozen.push(FrozenCard {
                    card: stack.base,
                    remaining_turns: 1,
                });
                for joker in stack.jokers {
                    self.scrap.push(joker.card);
                }
            }
            OneOffTarget::Jack { card } => {
                self.return_top_jack(card)?;
            }
            OneOffTarget::Joker { card } => {
                self.return_top_joker(card)?;
            }
            _ => return Err(RuleError::InvalidAction),
        }
        Ok(())
    }

    fn scrap_top_jack(&mut self, jack: Card) -> Result<(), RuleError> {
        for seat in 0..PLAYER_COUNT {
            for idx in 0..self.players[seat as usize].points.len() {
                if let Some(top) = self.players[seat as usize].points[idx].jacks.last()
                    && top.card == jack
                {
                    let target_owner = self.players[seat as usize].points[idx].controller();
                    if !self.can_target(target_owner, TargetKind::Jack, jack) {
                        return Err(RuleError::InvalidAction);
                    }
                    let mut stack = self.players[seat as usize].points.remove(idx);
                    let removed = stack.jacks.pop().expect("jack exists");
                    self.scrap.push(removed.card);
                    let new_owner = stack.controller();
                    self.players[new_owner as usize].points.push(stack);
                    return Ok(());
                }
            }
        }
        Err(RuleError::InvalidAction)
    }

    fn return_top_jack(&mut self, jack: Card) -> Result<(), RuleError> {
        for seat in 0..PLAYER_COUNT {
            for idx in 0..self.players[seat as usize].points.len() {
                if let Some(top) = self.players[seat as usize].points[idx].jacks.last()
                    && top.card == jack
                {
                    let target_owner = self.players[seat as usize].points[idx].controller();
                    if !self.can_target(target_owner, TargetKind::Jack, jack) {
                        return Err(RuleError::InvalidAction);
                    }
                    let mut stack = self.players[seat as usize].points.remove(idx);
                    let removed = stack.jacks.pop().expect("jack exists");
                    self.players[removed.owner as usize].hand.push(removed.card);
                    self.players[removed.owner as usize]
                        .frozen
                        .push(FrozenCard {
                            card: removed.card,
                            remaining_turns: 1,
                        });
                    let new_owner = stack.controller();
                    self.players[new_owner as usize].points.push(stack);
                    return Ok(());
                }
            }
        }
        Err(RuleError::InvalidAction)
    }

    fn scrap_top_joker(&mut self, joker: Card) -> Result<(), RuleError> {
        for seat in 0..PLAYER_COUNT {
            for idx in 0..self.players[seat as usize].royals.len() {
                if let Some(top) = self.players[seat as usize].royals[idx].jokers.last()
                    && top.card == joker
                {
                    let target_owner = self.players[seat as usize].royals[idx].controller();
                    if !self.can_target(target_owner, TargetKind::Joker, joker) {
                        return Err(RuleError::InvalidAction);
                    }
                    let mut stack = self.players[seat as usize].royals.remove(idx);
                    let removed = stack.jokers.pop().expect("joker exists");
                    self.scrap.push(removed.card);
                    let new_owner = stack.controller();
                    self.players[new_owner as usize].royals.push(stack);
                    return Ok(());
                }
            }
        }
        Err(RuleError::InvalidAction)
    }

    fn return_top_joker(&mut self, joker: Card) -> Result<(), RuleError> {
        for seat in 0..PLAYER_COUNT {
            for idx in 0..self.players[seat as usize].royals.len() {
                if let Some(top) = self.players[seat as usize].royals[idx].jokers.last()
                    && top.card == joker
                {
                    let target_owner = self.players[seat as usize].royals[idx].controller();
                    if !self.can_target(target_owner, TargetKind::Joker, joker) {
                        return Err(RuleError::InvalidAction);
                    }
                    let mut stack = self.players[seat as usize].royals.remove(idx);
                    let removed = stack.jokers.pop().expect("joker exists");
                    self.players[removed.owner as usize].hand.push(removed.card);
                    self.players[removed.owner as usize]
                        .frozen
                        .push(FrozenCard {
                            card: removed.card,
                            remaining_turns: 1,
                        });
                    let new_owner = stack.controller();
                    self.players[new_owner as usize].royals.push(stack);
                    return Ok(());
                }
            }
        }
        Err(RuleError::InvalidAction)
    }

    fn scrap_all_points(&mut self) {
        for seat in 0..PLAYER_COUNT {
            let stacks = std::mem::take(&mut self.players[seat as usize].points);
            for stack in stacks {
                self.scrap.push(stack.base);
                for jack in stack.jacks {
                    self.scrap.push(jack.card);
                }
            }
        }
    }

    fn scrap_all_royals(&mut self) {
        for seat in 0..PLAYER_COUNT {
            let stacks = std::mem::take(&mut self.players[seat as usize].royals);
            for stack in stacks {
                self.scrap.push(stack.base);
                for joker in stack.jokers {
                    self.scrap.push(joker.card);
                }
            }
        }
        for seat in 0..PLAYER_COUNT {
            for stack in &mut self.players[seat as usize].points {
                for jack in stack.jacks.drain(..) {
                    self.scrap.push(jack.card);
                }
            }
        }
    }

    fn reveal_top_two(&mut self) -> Vec<Card> {
        let mut revealed = Vec::new();
        if let Some(card) = self.deck.first().copied() {
            revealed.push(card);
        }
        if let Some(card) = self.deck.get(1).copied() {
            revealed.push(card);
        }
        for _ in 0..revealed.len() {
            self.deck.remove(0);
        }
        revealed
    }

    fn available_cards(&self, seat: Seat) -> Vec<Card> {
        let mut frozen_counts = std::collections::HashMap::<Card, u8>::new();
        for frozen in &self.players[seat as usize].frozen {
            *frozen_counts.entry(frozen.card).or_insert(0) += 1;
        }
        let mut available = Vec::new();
        for card in &self.players[seat as usize].hand {
            if let Some(count) = frozen_counts.get_mut(card)
                && *count > 0
            {
                *count -= 1;
                continue;
            }
            available.push(*card);
        }
        available
    }

    fn remove_from_hand(&mut self, seat: Seat, card: Card) -> Result<(), RuleError> {
        let hand = &mut self.players[seat as usize].hand;
        if let Some(idx) = hand.iter().position(|c| *c == card) {
            hand.remove(idx);
            if let Some(frozen_idx) = self.players[seat as usize]
                .frozen
                .iter()
                .position(|f| f.card == card)
            {
                self.players[seat as usize].frozen.remove(frozen_idx);
            }
            Ok(())
        } else {
            Err(RuleError::InvalidAction)
        }
    }

    fn finish_turn(&mut self, seat: Seat) {
        self.decrement_frozen(seat);
        self.turn = next_seat(seat);
    }

    fn decrement_frozen(&mut self, seat: Seat) {
        let frozen = &mut self.players[seat as usize].frozen;
        for entry in frozen.iter_mut() {
            if entry.remaining_turns > 0 {
                entry.remaining_turns -= 1;
            }
        }
        frozen.retain(|f| f.remaining_turns > 0);
    }

    fn reset_pass_streak(&mut self) {
        self.pass_streak_start = None;
        self.pass_streak_len = 0;
    }

    fn advance_pass_streak(&mut self, seat: Seat) {
        if self.pass_streak_len == 0 {
            self.pass_streak_start = Some(seat);
            self.pass_streak_len = 1;
        } else {
            self.pass_streak_len += 1;
        }
    }

    fn is_stalemate(&self, seat: Seat) -> bool {
        if self.pass_streak_start != Some(seat) {
            return false;
        }
        self.pass_streak_len > PLAYER_COUNT
    }

    fn iter_point_targets(&self) -> Vec<(Seat, &PointStack)> {
        let mut res = Vec::new();
        for seat in 0..PLAYER_COUNT {
            for stack in &self.players[seat as usize].points {
                res.push((stack.controller(), stack));
            }
        }
        res
    }

    fn iter_royal_targets(&self) -> Vec<(Seat, &RoyalStack)> {
        let mut res = Vec::new();
        for seat in 0..PLAYER_COUNT {
            for stack in &self.players[seat as usize].royals {
                res.push((stack.controller(), stack));
            }
        }
        res
    }

    fn iter_jack_targets(&self) -> Vec<(Seat, &PointStack, Card)> {
        let mut res = Vec::new();
        for seat in 0..PLAYER_COUNT {
            for stack in &self.players[seat as usize].points {
                if let Some(jack) = stack.jacks.last() {
                    res.push((stack.controller(), stack, jack.card));
                }
            }
        }
        res
    }

    fn iter_joker_targets(&self) -> Vec<(Seat, &RoyalStack, Card)> {
        let mut res = Vec::new();
        for seat in 0..PLAYER_COUNT {
            for stack in &self.players[seat as usize].royals {
                if let Some(joker) = stack.jokers.last() {
                    res.push((stack.controller(), stack, joker.card));
                }
            }
        }
        res
    }

    fn find_point_stack_by_base(&self, base: Card) -> Option<(Seat, usize)> {
        for seat in 0..PLAYER_COUNT {
            for (idx, stack) in self.players[seat as usize].points.iter().enumerate() {
                if stack.base == base {
                    return Some((seat, idx));
                }
            }
        }
        None
    }

    fn find_royal_stack_by_base(&self, base: Card) -> Option<(Seat, usize)> {
        for seat in 0..PLAYER_COUNT {
            for (idx, stack) in self.players[seat as usize].royals.iter().enumerate() {
                if stack.base == base {
                    return Some((seat, idx));
                }
            }
        }
        None
    }

    fn queen_count_for(&self, seat: Seat) -> usize {
        self.players[seat as usize]
            .royals
            .iter()
            .filter(|stack| {
                stack.controller() == seat
                    && matches!(
                        stack.base,
                        Card::Standard {
                            rank: Rank::Queen,
                            ..
                        }
                    )
            })
            .count()
    }

    fn player_has_glasses(&self, seat: Seat) -> bool {
        self.players[seat as usize].royals.iter().any(|stack| {
            stack.controller() == seat
                && matches!(
                    stack.base,
                    Card::Standard {
                        rank: Rank::Eight,
                        ..
                    }
                )
        })
    }

    fn can_target(&self, target_owner: Seat, kind: TargetKind, target_card: Card) -> bool {
        let queens = self.queen_count_for(target_owner);
        if queens == 0 {
            return true;
        }
        if queens >= 2 {
            return false;
        }
        matches!(kind, TargetKind::Royal)
            && matches!(
                target_card,
                Card::Standard {
                    rank: Rank::Queen,
                    ..
                }
            )
    }

    fn points_for(&self, seat: Seat) -> u8 {
        self.players[seat as usize]
            .points
            .iter()
            .filter(|stack| stack.controller() == seat)
            .map(|stack| stack.base.point_value())
            .sum()
    }

    fn kings_for(&self, seat: Seat) -> usize {
        self.players[seat as usize]
            .royals
            .iter()
            .filter(|stack| {
                stack.controller() == seat
                    && matches!(
                        stack.base,
                        Card::Standard {
                            rank: Rank::King,
                            ..
                        }
                    )
            })
            .count()
    }

    fn threshold_for(&self, seat: Seat) -> u8 {
        match self.kings_for(seat) {
            0 => 14,
            1 => 9,
            2 => 5,
            _ => 0,
        }
    }

    fn check_winner(&self) -> Option<Winner> {
        if matches!(self.phase, Phase::GameOver) {
            return self.winner.clone();
        }
        for seat in 0..PLAYER_COUNT {
            if self.points_for(seat) >= self.threshold_for(seat) {
                return Some(Winner::Seat(seat));
            }
        }
        None
    }
}

#[derive(Clone, Debug, Error)]
pub enum RuleError {
    #[error("not your turn")]
    NotYourTurn,
    #[error("illegal action")]
    IllegalAction,
    #[error("invalid action")]
    InvalidAction,
    #[error("game over")]
    GameOver,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Event {
    GameOver(Winner),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TargetKind {
    Point,
    Royal,
    Jack,
    Joker,
}

fn is_point_card(card: Card) -> bool {
    matches!(
        card,
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

fn next_seat(seat: Seat) -> Seat {
    (seat + 1) % PLAYER_COUNT
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum PublicCard {
    Hidden,
    Known(String),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PointStackView {
    pub base: String,
    pub controller: Seat,
    pub jacks: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoyalStackView {
    pub base: String,
    pub controller: Seat,
    pub jokers: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerView {
    pub seat: Seat,
    pub hand: Vec<PublicCard>,
    pub points: Vec<PointStackView>,
    pub royals: Vec<RoyalStackView>,
    pub frozen: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublicView {
    pub seat: Seat,
    pub turn: Seat,
    pub phase: PhaseView,
    pub deck_count: usize,
    pub scrap: Vec<String>,
    pub players: Vec<PlayerView>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_event: Option<LastEventView>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LastEventView {
    pub actor: Seat,
    pub action_kind: String,
    pub change: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_token: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_zone: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_token: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_seat: Option<Seat>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub oneoff_rank: Option<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CounterTwoView {
    pub seat: Seat,
    pub card: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum PhaseView {
    Main,
    Countering {
        next_seat: Seat,
        base_player: Seat,
        oneoff: Action,
        twos: Vec<CounterTwoView>,
    },
    ResolvingThree {
        seat: Seat,
    },
    ResolvingFour {
        seat: Seat,
        remaining: u8,
    },
    ResolvingFive {
        seat: Seat,
    },
    ResolvingSeven {
        seat: Seat,
        revealed: usize,
        revealed_cards: Vec<String>,
    },
    GameOver,
}

impl Phase {
    fn view(&self, viewer: Seat) -> PhaseView {
        match self {
            Phase::Main => PhaseView::Main,
            Phase::Countering(counter) => PhaseView::Countering {
                next_seat: counter.next_seat,
                base_player: counter.base_player,
                oneoff: counter.oneoff.clone(),
                twos: counter
                    .twos
                    .iter()
                    .map(|(seat, card)| CounterTwoView {
                        seat: *seat,
                        card: card.to_token(),
                    })
                    .collect(),
            },
            Phase::ResolvingThree { seat, .. } => PhaseView::ResolvingThree { seat: *seat },
            Phase::ResolvingFour {
                seat, remaining, ..
            } => PhaseView::ResolvingFour {
                seat: *seat,
                remaining: *remaining,
            },
            Phase::ResolvingFive { seat, .. } => PhaseView::ResolvingFive { seat: *seat },
            Phase::ResolvingSeven { seat, revealed, .. } => PhaseView::ResolvingSeven {
                seat: *seat,
                revealed: revealed.len(),
                revealed_cards: if *seat == viewer {
                    revealed.iter().map(|card| card.to_token()).collect()
                } else {
                    Vec::new()
                },
            },
            Phase::GameOver => PhaseView::GameOver,
        }
    }
}
