//! Honey is a mate-only analyzer.
//!
//! It deliberately has no positional or material evaluation.  Its search is a
//! depth-bounded AND/OR proof-number search: turns played by the prospective
//! winner are OR nodes, and turns played by the defender are AND nodes.  Bounds
//! grow exponentially until a mate is proved, then binary search establishes
//! the shortest minimax mate distance.

use snipe_core::{
    Action, ActionWriter, Analyzer, Animal, AnimalDrop, AnimalStep, Card, Evaluation,
    EvaluationEstimate, MateInN, OptimalOutcome, Player, Rank, SnipeStep, State, StepDirection,
};
use std::{
    cmp::Reverse,
    collections::HashMap,
    hash::{BuildHasherDefault, Hash, Hasher},
    sync::Arc,
};

const INF: u64 = 1_u64 << 60;
const FIRST_BOUND: u8 = 4;
const MAX_BOUND: u8 = 64;
const WORK_PER_TICK: usize = 64;
const FORCING_QUIET_PLIES: u8 = 4;
const MAX_DEFENSE_FIRST_ACTIONS: usize = 96;
const MAX_FORCING_FIRST_ACTIONS: usize = 20;

const ANIMALS: [Animal; 16] = [
    Animal::Mouse,
    Animal::Ox,
    Animal::Tiger,
    Animal::Rabbit,
    Animal::Dragon,
    Animal::Snake,
    Animal::Horse,
    Animal::Ram,
    Animal::Monkey,
    Animal::Rooster,
    Animal::Dog,
    Animal::Boar,
    Animal::Fish,
    Animal::Elephant,
    Animal::Squid,
    Animal::Frog,
];

const RETREATERS: u16 = bit(0) | bit(3) | bit(5) | bit(7) | bit(11) | bit(14);

// Element masks, indexed Fire, Water, Earth, Air.  Captures need one animal
// from each arity for the same element; the entering animal participates too.
const UNARY: [u16; 4] = [
    bit(9) | bit(11) | bit(14),
    bit(1) | bit(3) | bit(10),
    bit(0) | bit(5) | bit(8),
    bit(6) | bit(7) | bit(15),
];
const BINARY: [u16; 4] = [
    bit(0) | bit(6) | bit(10),
    bit(5) | bit(14) | bit(15),
    bit(1) | bit(7) | bit(11),
    bit(3) | bit(8) | bit(9),
];
const TERNARY: [u16; 4] = [bit(2), bit(12), bit(13), bit(4)];

const fn bit(index: u8) -> u16 {
    1_u16 << index
}

type FastMap<K, V> = HashMap<K, V, BuildHasherDefault<MateHasher>>;

#[derive(Default)]
struct MateHasher(u64);

impl Hasher for MateHasher {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, bytes: &[u8]) {
        let mut hash = self.0 ^ 0x517c_c1b7_2722_0a95;
        for &byte in bytes {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x100_0000_01b3);
        }
        self.0 = hash ^ (hash >> 29);
    }

    fn write_u64(&mut self, value: u64) {
        let mut mixed = value.wrapping_add(self.0 ^ 0x9e37_79b9_7f4a_7c15);
        mixed = (mixed ^ (mixed >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        mixed = (mixed ^ (mixed >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        self.0 = mixed ^ (mixed >> 31);
    }

    fn write_u8(&mut self, value: u8) {
        self.write_u64(u64::from(value));
    }
}

/// Two canonical four-bit cards per animal.  A nibble stores a three-bit
/// location (reserve or ranks 1..=6) plus one allegiance bit.  This is compact
/// enough that position keys remain cheap to copy and hash.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Position {
    animals_lo: u64,
    animals_hi: u64,
    snipes: u8,
    turn: u8,
    leading: u8,
}

impl Hash for Position {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u64(self.animals_lo);
        state.write_u64(self.animals_hi);
        state.write_u64(
            u64::from(self.snipes) | (u64::from(self.turn) << 8) | (u64::from(self.leading) << 16),
        );
    }
}

impl Position {
    fn from_core(state: &State) -> Self {
        let locations = [
            state.reserves,
            state.r1,
            state.r2,
            state.r3,
            state.r4,
            state.r5,
            state.r6,
        ];
        let mut position = Self {
            animals_lo: 0,
            animals_hi: 0,
            snipes: 0,
            turn: player_bit(state.active_player),
            leading: encode_leading(state.leading_action),
        };

        for (animal_index, animal) in ANIMALS.into_iter().enumerate() {
            let mut tokens = [0_u8; 2];
            let mut token_index = 0;
            for (location, cards) in locations.into_iter().enumerate() {
                for owner in [Player::Alpha, Player::Beta] {
                    for _ in 0..cards.count(Card::Animal(animal), owner) {
                        assert!(token_index < 2, "an animal has more than two cards");
                        tokens[token_index] = token(owner, location as u8);
                        token_index += 1;
                    }
                }
            }
            assert_eq!(token_index, 2, "an animal must have exactly two cards");
            tokens.sort_unstable();
            position.set_animal_byte(animal_index, tokens[0] | (tokens[1] << 4));
        }

        for owner in [Player::Alpha, Player::Beta] {
            let mut found = None;
            for (location, cards) in locations.into_iter().enumerate() {
                if cards.count(Card::Snipe, owner) != 0 {
                    found = Some(location as u8);
                    break;
                }
            }
            let location = found.expect("each player has exactly one snipe");
            position.set_snipe_location(owner, location);
        }
        position
    }

    fn active_player(self) -> Player {
        if self.turn == 0 {
            Player::Alpha
        } else {
            Player::Beta
        }
    }

    fn animal_byte(self, index: usize) -> u8 {
        if index < 8 {
            (self.animals_lo >> (index * 8)) as u8
        } else {
            (self.animals_hi >> ((index - 8) * 8)) as u8
        }
    }

    fn set_animal_byte(&mut self, index: usize, value: u8) {
        if index < 8 {
            let shift = index * 8;
            self.animals_lo =
                (self.animals_lo & !(0xff_u64 << shift)) | (u64::from(value) << shift);
        } else {
            let shift = (index - 8) * 8;
            self.animals_hi =
                (self.animals_hi & !(0xff_u64 << shift)) | (u64::from(value) << shift);
        }
    }

    fn snipe_location(self, player: Player) -> u8 {
        match player {
            Player::Alpha => self.snipes & 0x0f,
            Player::Beta => self.snipes >> 4,
        }
    }

    fn set_snipe_location(&mut self, player: Player, location: u8) {
        match player {
            Player::Alpha => self.snipes = (self.snipes & 0xf0) | location,
            Player::Beta => self.snipes = (self.snipes & 0x0f) | (location << 4),
        }
    }

    fn captured_snipe_winner(self) -> Option<Player> {
        if self.snipe_location(Player::Beta) == 0 {
            Some(Player::Alpha)
        } else if self.snipe_location(Player::Alpha) == 0 {
            Some(Player::Beta)
        } else {
            None
        }
    }

    /// Returns a legal animal action that captures the opposing snipe now.
    /// This is the Snipe Hunt analogue of an immediate checking mate and is
    /// cheap to test without expanding unrelated legal moves.
    fn winning_capture(self) -> Option<Action> {
        if self.captured_snipe_winner().is_some() {
            return None;
        }
        let active = self.active_player();
        let destination = self.snipe_location(active.opponent());
        if destination == 0 {
            return None;
        }
        let view = BoardView::new(self);
        let leading = decode_leading(self.leading);
        for direction in [StepDirection::Advance, StepDirection::Retreat] {
            let Some(source) = step_source(active, direction, destination) else {
                continue;
            };
            let mut animals = view.friendly(active, source as usize);
            if direction == StepDirection::Retreat {
                animals &= RETREATERS;
            }
            while animals != 0 {
                let animal = animals.trailing_zeros() as u8;
                animals &= animals - 1;
                if let Some(leading) = leading
                    && animal_index(leading.actor) == animal as usize
                    && rank_number(leading.destination) == source
                    && view.friendly_twins(active, source as usize) & bit(animal) == 0
                {
                    continue;
                }
                if activates_triplet(animal as usize, view.presence[destination as usize]) {
                    let action = Action::AnimalStep(AnimalStep {
                        actor: ANIMALS[animal as usize],
                        direction,
                        destination: number_rank(destination),
                    });
                    if self.apply_known(action, true).captured_snipe_winner() == Some(active) {
                        return Some(action);
                    }
                }
            }
        }
        None
    }

    #[cfg(test)]
    fn apply(self, action: Action) -> Self {
        let capture = match action {
            Action::AnimalStep(step) => {
                let destination = rank_number(step.destination);
                activates_triplet(
                    animal_index(step.actor),
                    BoardView::new(self).presence[destination as usize],
                )
            }
            Action::SnipeStep(_) | Action::Drop(_) => false,
        };
        self.apply_known(action, capture)
    }

    fn apply_known(self, action: Action, capture: bool) -> Self {
        let active = self.active_player();
        let mut next = self;
        match action {
            Action::SnipeStep(step) => {
                next.set_snipe_location(active, rank_number(step.destination));
                next.turn ^= 1;
            }
            Action::Drop(drop) => {
                next.move_animal(
                    animal_index(drop.actor),
                    active,
                    0,
                    rank_number(drop.destination),
                );
                next.turn ^= 1;
            }
            Action::AnimalStep(step) => {
                let destination = rank_number(step.destination);
                let source = step_source(active, step.direction, destination)
                    .expect("generated step has an in-range source");
                if capture {
                    next.capture_rank(destination, active);
                }
                next.move_animal(animal_index(step.actor), active, source, destination);
                if self.leading == 0 {
                    next.leading = encode_leading(Some(step));
                } else {
                    next.leading = 0;
                    next.turn ^= 1;
                }
            }
        }
        next
    }

    fn move_animal(&mut self, animal: usize, owner: Player, source: u8, destination: u8) {
        let mut pair = self.animal_byte(animal);
        let sought = token(owner, source);
        let replacement = token(owner, destination);
        let first = pair & 0x0f;
        let second = pair >> 4;
        if first == sought {
            pair = replacement | (second << 4);
        } else {
            assert_eq!(second, sought, "generated move has its actor at the source");
            pair = first | (replacement << 4);
        }
        let first = pair & 0x0f;
        let second = pair >> 4;
        self.set_animal_byte(animal, first.min(second) | (first.max(second) << 4));
    }

    fn capture_rank(&mut self, rank: u8, captor: Player) {
        for animal in 0..16 {
            let pair = self.animal_byte(animal);
            let mut first = pair & 0x0f;
            let mut second = pair >> 4;
            if token_location(first) == rank {
                first = token(captor, 0);
            }
            if token_location(second) == rank {
                second = token(captor, 0);
            }
            self.set_animal_byte(animal, first.min(second) | (first.max(second) << 4));
        }
        for snipe in [Player::Alpha, Player::Beta] {
            if self.snipe_location(snipe) == rank {
                self.set_snipe_location(snipe, 0);
            }
        }
    }
}

#[derive(Clone, Copy)]
struct BoardView {
    alpha: [u16; 7],
    beta: [u16; 7],
    alpha_twins: [u16; 7],
    beta_twins: [u16; 7],
    presence: [u16; 7],
    card_count: [u8; 7],
    reserve_count: [u8; 2],
}

impl BoardView {
    fn new(position: Position) -> Self {
        let mut view = Self {
            alpha: [0; 7],
            beta: [0; 7],
            alpha_twins: [0; 7],
            beta_twins: [0; 7],
            presence: [0; 7],
            card_count: [0; 7],
            reserve_count: [0; 2],
        };
        for animal in 0..16 {
            let pair = position.animal_byte(animal);
            let first = pair & 0x0f;
            let second = pair >> 4;
            view.add_token(animal, first);
            view.add_token(animal, second);
            if first == second {
                let location = token_location(first) as usize;
                if token_player(first) == Player::Alpha {
                    view.alpha_twins[location] |= bit(animal as u8);
                } else {
                    view.beta_twins[location] |= bit(animal as u8);
                }
            }
        }
        for player in [Player::Alpha, Player::Beta] {
            view.card_count[position.snipe_location(player) as usize] += 1;
        }
        view
    }

    fn add_token(&mut self, animal: usize, value: u8) {
        let location = token_location(value) as usize;
        let animal_bit = bit(animal as u8);
        if token_player(value) == Player::Alpha {
            self.alpha[location] |= animal_bit;
            if location == 0 {
                self.reserve_count[0] += 1;
            }
        } else {
            self.beta[location] |= animal_bit;
            if location == 0 {
                self.reserve_count[1] += 1;
            }
        }
        self.presence[location] |= animal_bit;
        self.card_count[location] += 1;
    }

    fn friendly(self, player: Player, location: usize) -> u16 {
        match player {
            Player::Alpha => self.alpha[location],
            Player::Beta => self.beta[location],
        }
    }

    fn friendly_twins(self, player: Player, location: usize) -> u16 {
        match player {
            Player::Alpha => self.alpha_twins[location],
            Player::Beta => self.beta_twins[location],
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct ScoredAction {
    action: Action,
    score: i32,
    capture: bool,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct ChoiceLine {
    first: Action,
    second: Option<Action>,
}

#[derive(Clone, Copy, Debug)]
struct SearchChoice {
    line: ChoiceLine,
    next: Position,
    consumes_ply: bool,
    score: i32,
}

#[derive(Clone, Copy, Debug)]
struct OnePlyResolution {
    target_wins: bool,
    best: ChoiceLine,
}

#[derive(Default)]
struct MoveCache {
    actions: FastMap<Position, Arc<[ScoredAction]>>,
    action_choices: FastMap<Position, Arc<[SearchChoice]>>,
    complete_turns: FastMap<Position, Arc<[SearchChoice]>>,
    evasion_turns: FastMap<Position, Arc<[SearchChoice]>>,
    forcing_turns: FastMap<Position, Arc<[SearchChoice]>>,
    capture_mates: FastMap<Position, Option<ChoiceLine>>,
    one_ply_resolutions: FastMap<Position, OnePlyResolution>,
}

impl MoveCache {
    fn actions(&mut self, position: Position) -> Arc<[ScoredAction]> {
        if let Some(actions) = self.actions.get(&position) {
            return Arc::clone(actions);
        }
        let actions: Arc<[ScoredAction]> = generate_actions(position).into();
        self.actions.insert(position, Arc::clone(&actions));
        actions
    }

    fn search_choices(&mut self, position: Position, complete_turn: bool) -> Arc<[SearchChoice]> {
        if complete_turn {
            return self.turn_choices(position);
        }
        if let Some(choices) = self.action_choices.get(&position) {
            return Arc::clone(choices);
        }
        let choices: Arc<[SearchChoice]> = self
            .actions(position)
            .iter()
            .map(|candidate| SearchChoice {
                line: ChoiceLine {
                    first: candidate.action,
                    second: None,
                },
                next: position.apply_known(candidate.action, candidate.capture),
                consumes_ply: position.leading == 0,
                score: candidate.score,
            })
            .collect::<Vec<_>>()
            .into();
        self.action_choices.insert(position, Arc::clone(&choices));
        if position.leading != 0 {
            // The action-level choices contain everything DFPN needs for the
            // remainder of this already-started ply.  Retaining the parallel
            // ScoredAction vector roughly doubles the dominant deep-search
            // cache for no benefit.
            self.actions.remove(&position);
        }
        choices
    }

    fn can_expand_complete_turn(&mut self, position: Position) -> bool {
        self.actions(position).len() <= MAX_DEFENSE_FIRST_ACTIONS
    }

    fn can_expand_forcing_turn(&mut self, position: Position) -> bool {
        self.actions(position).len() <= MAX_FORCING_FIRST_ACTIONS
    }

    /// Resolves the special one-ply horizon where the opponent is to move.
    /// Usually any completed move survives that ply.  Snipe Hunt also permits
    /// a player to lose during its own animal turn by having no legal second
    /// action, and simultaneous snipe captures have an asymmetric winner, so
    /// a chess-style parity shortcut alone would be unsound.
    fn resolve_opponent_ply(&mut self, position: Position) -> OnePlyResolution {
        if let Some(resolution) = self.one_ply_resolutions.get(&position) {
            return *resolution;
        }
        debug_assert_eq!(position.leading, 0);
        let active = position.active_player();
        let target = active.opponent();
        let first_actions = self.actions(position);
        let mut representative_loss = None;
        for first in first_actions.iter() {
            let first_line = ChoiceLine {
                first: first.action,
                second: None,
            };
            let after_first = position.apply_known(first.action, first.capture);
            if let Some(winner) = after_first.captured_snipe_winner() {
                if winner != target {
                    let resolution = OnePlyResolution {
                        target_wins: false,
                        best: first_line,
                    };
                    return self.store_one_ply_resolution(position, resolution);
                }
                representative_loss.get_or_insert(first_line);
                continue;
            }
            if after_first.active_player() != active {
                let resolution = OnePlyResolution {
                    target_wins: false,
                    best: first_line,
                };
                return self.store_one_ply_resolution(position, resolution);
            }

            let second_actions = self.actions(after_first);
            if second_actions.is_empty() {
                self.actions.remove(&after_first);
                representative_loss.get_or_insert(first_line);
                continue;
            }
            for second in second_actions.iter() {
                let line = ChoiceLine {
                    first: first.action,
                    second: Some(second.action),
                };
                let after_second = after_first.apply_known(second.action, second.capture);
                if after_second.captured_snipe_winner() != Some(target) {
                    let resolution = OnePlyResolution {
                        target_wins: false,
                        best: line,
                    };
                    self.actions.remove(&after_first);
                    return self.store_one_ply_resolution(position, resolution);
                }
                representative_loss.get_or_insert(line);
            }
            self.actions.remove(&after_first);
        }
        let resolution = OnePlyResolution {
            target_wins: true,
            best: representative_loss.expect("a legal opponent ply has a representative line"),
        };
        self.store_one_ply_resolution(position, resolution)
    }

    fn store_one_ply_resolution(
        &mut self,
        position: Position,
        resolution: OnePlyResolution,
    ) -> OnePlyResolution {
        self.actions.remove(&position);
        self.one_ply_resolutions.insert(position, resolution);
        resolution
    }

    /// Canonical completed plies for defender nodes. Sequential animal steps
    /// that reach an identical full state are one game-theoretic choice, even
    /// when both action orders are legal.
    fn turn_choices(&mut self, position: Position) -> Arc<[SearchChoice]> {
        if let Some(choices) = self.complete_turns.get(&position) {
            return Arc::clone(choices);
        }
        debug_assert_eq!(position.leading, 0);
        let first_actions = self.actions(position);
        let mut choices = Vec::with_capacity(first_actions.len() * 4);
        let mut by_result = FastMap::<Position, usize>::default();
        for first in first_actions.iter() {
            let after_first = position.apply_known(first.action, first.capture);
            if after_first.captured_snipe_winner().is_some()
                || after_first.active_player() != position.active_player()
            {
                insert_unique_choice(
                    &mut choices,
                    &mut by_result,
                    SearchChoice {
                        line: ChoiceLine {
                            first: first.action,
                            second: None,
                        },
                        next: after_first,
                        consumes_ply: true,
                        score: first.score,
                    },
                );
                continue;
            }

            let second_actions = self.actions(after_first);
            if second_actions.is_empty() {
                self.actions.remove(&after_first);
                insert_unique_choice(
                    &mut choices,
                    &mut by_result,
                    SearchChoice {
                        line: ChoiceLine {
                            first: first.action,
                            second: None,
                        },
                        next: after_first,
                        consumes_ply: true,
                        score: first.score,
                    },
                );
                continue;
            }
            for second in second_actions.iter() {
                insert_unique_choice(
                    &mut choices,
                    &mut by_result,
                    SearchChoice {
                        line: ChoiceLine {
                            first: first.action,
                            second: Some(second.action),
                        },
                        next: after_first.apply_known(second.action, second.capture),
                        consumes_ply: true,
                        score: first.score.saturating_add(second.score),
                    },
                );
            }
            self.actions.remove(&after_first);
        }
        choices.sort_unstable_by_key(|choice| Reverse(choice.score));
        let choices: Arc<[SearchChoice]> = choices.into();
        self.complete_turns.insert(position, Arc::clone(&choices));
        choices
    }

    /// Finds a concrete snipe-capturing line inside the current ply.  At a
    /// defender layer this partitions completed moves into true evasions and
    /// moves that leave the explicit mate threat intact.
    fn capture_mate(&mut self, position: Position) -> Option<ChoiceLine> {
        if let Some(cached) = self.capture_mates.get(&position) {
            return *cached;
        }
        let found = if let Some(first) = position.winning_capture() {
            Some(ChoiceLine {
                first,
                second: None,
            })
        } else if position.leading == 0 {
            let active = position.active_player();
            let actions = self.actions(position);
            actions.iter().find_map(|first| {
                if !matches!(first.action, Action::AnimalStep(_)) {
                    return None;
                }
                let after_first = position.apply_known(first.action, first.capture);
                if after_first.active_player() != active
                    || after_first.captured_snipe_winner().is_some()
                {
                    return None;
                }
                after_first.winning_capture().map(|second| ChoiceLine {
                    first: first.action,
                    second: Some(second),
                })
            })
        } else {
            None
        };
        self.capture_mates.insert(position, found);
        found
    }

    /// Candidate attacking plies for threat-space search.  Every retained
    /// choice either wins immediately or leaves a concrete capture-mate on the
    /// following attacking ply.  This is deliberately incomplete as move
    /// generation, but any proof built from it remains fully sound.
    fn forcing_choices(&mut self, position: Position) -> Arc<[SearchChoice]> {
        if let Some(choices) = self.forcing_turns.get(&position) {
            return Arc::clone(choices);
        }
        debug_assert_eq!(position.leading, 0);
        if !self.can_expand_forcing_turn(position) {
            let empty: Arc<[SearchChoice]> = Arc::from([]);
            self.forcing_turns.insert(position, Arc::clone(&empty));
            return empty;
        }
        let attacker = position.active_player();
        let turns = self.turn_choices(position);
        let mut forcing = Vec::new();
        for choice in turns.iter().copied() {
            if choice.next.captured_snipe_winner() == Some(attacker) {
                forcing.push(choice);
                continue;
            }
            if !has_legal_action(choice.next) {
                forcing.push(choice);
                continue;
            }
            // A nominal threat that lets the defender capture our snipe on
            // the reply cannot be part of a forced mate.  This is the same
            // cheap "must answer check" filter used at ordinary search nodes.
            if self.capture_mate(choice.next).is_some() {
                continue;
            }
            let mut threat_position = choice.next;
            threat_position.turn = player_bit(attacker);
            threat_position.leading = 0;
            if self.capture_mate(threat_position).is_some() {
                forcing.push(choice);
            }
        }
        let forcing: Arc<[SearchChoice]> = forcing.into();
        self.forcing_turns.insert(position, Arc::clone(&forcing));
        forcing
    }

    /// Cheap full-ply filter, analogous to legal check evasions in chess.  A
    /// completed move that leaves a witnessed capture-mate is strictly
    /// dominated whenever a real evasion exists.  This is exact at both OR
    /// and AND nodes: a move that permits the opponent to win next ply cannot
    /// help its mover force mate, while at a defender node it is already a
    /// solved branch of the attacker's proof.
    fn evasion_choices(&mut self, position: Position) -> Arc<[SearchChoice]> {
        if let Some(choices) = self.evasion_turns.get(&position) {
            return Arc::clone(choices);
        }
        let defender = position.active_player();
        let attacker = defender.opponent();
        let turns = self.turn_choices(position);
        let mut evasions = Vec::new();
        let mut representative_loss = None;
        for choice in turns.iter().copied() {
            let winner = choice.next.captured_snipe_winner().or_else(|| {
                (!has_legal_action(choice.next)).then(|| choice.next.active_player().opponent())
            });
            let loses_immediately = winner == Some(attacker)
                || (winner.is_none()
                    && choice.next.active_player() == attacker
                    && self.capture_mate(choice.next).is_some());
            if loses_immediately {
                representative_loss.get_or_insert(choice);
            } else {
                evasions.push(choice);
            }
        }
        if evasions.is_empty()
            && let Some(loss) = representative_loss
        {
            evasions.push(loss);
        }
        let evasions: Arc<[SearchChoice]> = evasions.into();
        self.evasion_turns.insert(position, Arc::clone(&evasions));
        evasions
    }
}

fn insert_unique_choice(
    choices: &mut Vec<SearchChoice>,
    by_result: &mut FastMap<Position, usize>,
    candidate: SearchChoice,
) {
    if let Some(&index) = by_result.get(&candidate.next) {
        if candidate.score > choices[index].score {
            choices[index] = candidate;
        }
    } else {
        by_result.insert(candidate.next, choices.len());
        choices.push(candidate);
    }
}

fn generate_actions(position: Position) -> Vec<ScoredAction> {
    if position.captured_snipe_winner().is_some() {
        return Vec::new();
    }
    let active = position.active_player();
    let enemy = active.opponent();
    let view = BoardView::new(position);
    let active_index = usize::from(active == Player::Beta);
    let mut actions = Vec::with_capacity(96);

    if position.leading == 0 {
        let source = position.snipe_location(active);
        if source != 0 && view.card_count[source as usize] > 1 {
            for destination in [advance(active, source), retreat(active, source)]
                .into_iter()
                .flatten()
            {
                let action = Action::SnipeStep(SnipeStep {
                    destination: number_rank(destination),
                });
                actions.push(ScoredAction {
                    action,
                    score: action_score(position, action, false),
                    capture: false,
                });
            }
        }

        if view.reserve_count[active_index] > 1 {
            let mut animals = view.friendly(active, 0);
            while animals != 0 {
                let animal = animals.trailing_zeros() as u8;
                animals &= animals - 1;
                for destination in 1..=6 {
                    if RETREATERS & bit(animal) == 0 || legal_retreater_drop(active, destination) {
                        let action = Action::Drop(AnimalDrop {
                            actor: ANIMALS[animal as usize],
                            destination: number_rank(destination),
                        });
                        actions.push(ScoredAction {
                            action,
                            score: action_score(position, action, false),
                            capture: false,
                        });
                    }
                }
            }
        }
    }

    let leading = decode_leading(position.leading);
    for source in 1..=6 {
        let mut animals = view.friendly(active, source as usize);
        while animals != 0 {
            let animal = animals.trailing_zeros() as u8;
            animals &= animals - 1;
            let directions = [StepDirection::Advance, StepDirection::Retreat];
            for direction in directions {
                if direction == StepDirection::Retreat && RETREATERS & bit(animal) == 0 {
                    continue;
                }
                let Some(destination) = step_destination(active, direction, source) else {
                    continue;
                };
                if let Some(leading) = leading
                    && animal_index(leading.actor) == animal as usize
                    && rank_number(leading.destination) == source
                    && view.friendly_twins(active, source as usize) & bit(animal) == 0
                {
                    continue;
                }

                let capture =
                    activates_triplet(animal as usize, view.presence[destination as usize]);
                let enemy_snipe = position.snipe_location(enemy) == destination;
                let friendly_snipe = position.snipe_location(active) == destination;
                if view.card_count[source as usize] <= 1 {
                    if !capture || !enemy_snipe {
                        continue;
                    }
                } else if capture && friendly_snipe && !enemy_snipe {
                    continue;
                }

                let action = Action::AnimalStep(AnimalStep {
                    actor: ANIMALS[animal as usize],
                    direction,
                    destination: number_rank(destination),
                });
                actions.push(ScoredAction {
                    action,
                    score: action_score(position, action, capture),
                    capture,
                });
            }
        }
    }

    actions.sort_unstable_by_key(|action| Reverse(action.score));
    actions
}

fn has_legal_action(position: Position) -> bool {
    if position.captured_snipe_winner().is_some() {
        return false;
    }
    let active = position.active_player();
    let enemy = active.opponent();
    let view = BoardView::new(position);
    let active_index = usize::from(active == Player::Beta);

    if position.leading == 0 {
        let snipe = position.snipe_location(active);
        if snipe != 0
            && view.card_count[snipe as usize] > 1
            && (advance(active, snipe).is_some() || retreat(active, snipe).is_some())
        {
            return true;
        }
        if view.reserve_count[active_index] > 1 {
            return true;
        }
    }

    let leading = decode_leading(position.leading);
    for source in 1..=6 {
        let mut animals = view.friendly(active, source as usize);
        while animals != 0 {
            let animal = animals.trailing_zeros() as u8;
            animals &= animals - 1;
            for direction in [StepDirection::Advance, StepDirection::Retreat] {
                if direction == StepDirection::Retreat && RETREATERS & bit(animal) == 0 {
                    continue;
                }
                let Some(destination) = step_destination(active, direction, source) else {
                    continue;
                };
                if let Some(leading) = leading
                    && animal_index(leading.actor) == animal as usize
                    && rank_number(leading.destination) == source
                    && view.friendly_twins(active, source as usize) & bit(animal) == 0
                {
                    continue;
                }
                let capture =
                    activates_triplet(animal as usize, view.presence[destination as usize]);
                let enemy_snipe = position.snipe_location(enemy) == destination;
                let friendly_snipe = position.snipe_location(active) == destination;
                if view.card_count[source as usize] <= 1 {
                    if capture && enemy_snipe {
                        return true;
                    }
                } else if !capture || !friendly_snipe || enemy_snipe {
                    return true;
                }
            }
        }
    }
    false
}

/// Mate-only move ordering.  Scores are always from the acting player's point
/// of view, so both OR nodes and the defender's most-promising refutations are
/// searched first without introducing a non-mate evaluation.
fn action_score(position: Position, action: Action, capture: bool) -> i32 {
    let active = position.active_player();
    let enemy_snipe = i32::from(position.snipe_location(active.opponent()));
    let own_snipe = i32::from(position.snipe_location(active));
    match action {
        Action::AnimalStep(step) => {
            let destination = i32::from(rank_number(step.destination));
            let mut score = 4_000 - (destination - enemy_snipe).abs() * 180;
            if capture {
                score += 20_000;
                if destination == enemy_snipe {
                    score += 1_000_000;
                }
            }
            if destination == own_snipe {
                score -= 600;
            }
            if position.leading != 0 {
                score += 40;
            }
            score
        }
        Action::Drop(drop) => {
            let destination = i32::from(rank_number(drop.destination));
            1_000 - (destination - enemy_snipe).abs() * 80
        }
        Action::SnipeStep(step) => {
            let destination = i32::from(rank_number(step.destination));
            200 + (destination - enemy_snipe).abs() * 40 - (destination - own_snipe).abs()
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct SearchKey {
    position: Position,
    plies_left: u8,
    quiet_left: u8,
}

impl Hash for SearchKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.position.hash(state);
        state.write_u8(self.plies_left);
        state.write_u8(self.quiet_left);
    }
}

#[derive(Clone, Copy, Debug)]
struct Entry {
    proof: u64,
    disproof: u64,
    best: Option<ChoiceLine>,
}

impl Entry {
    const UNKNOWN: Self = Self {
        proof: 1,
        disproof: 1,
        best: None,
    };
}

struct MateSearch {
    target: Player,
    forcing_only: bool,
    table: FastMap<SearchKey, Entry>,
    lower: u8,
    upper: Option<u8>,
    active_bound: u8,
    exact: Option<u8>,
    exhausted: bool,
}

impl MateSearch {
    fn new(target: Player, forcing_only: bool) -> Self {
        Self {
            target,
            forcing_only,
            table: FastMap::default(),
            lower: 0,
            upper: None,
            active_bound: FIRST_BOUND,
            exact: None,
            exhausted: false,
        }
    }

    fn think(&mut self, root: Position, moves: &mut MoveCache, work: usize) {
        if self.exact.is_some() || self.exhausted {
            return;
        }
        let key = SearchKey {
            position: root,
            plies_left: self.active_bound,
            quiet_left: if self.forcing_only {
                FORCING_QUIET_PLIES
            } else {
                0
            },
        };
        let mut budget = work;
        self.dfpn(key, INF, INF, moves, &mut budget);
        let entry = self.entry(key, moves);
        if entry.proof == 0 {
            self.upper = Some(
                self.upper
                    .map_or(self.active_bound, |old| old.min(self.active_bound)),
            );
            if self.forcing_only {
                self.exact = Some(self.active_bound);
            } else {
                self.select_next_bound();
            }
        } else if entry.disproof == 0 {
            self.lower = self.lower.max(self.active_bound);
            self.select_next_bound();
        }
    }

    fn select_next_bound(&mut self) {
        if let Some(upper) = self.upper {
            if upper == self.lower.saturating_add(1) {
                self.exact = Some(upper);
            } else {
                self.active_bound = self.lower + (upper - self.lower) / 2;
            }
        } else if self.active_bound == MAX_BOUND {
            self.exhausted = true;
        } else {
            self.active_bound = self.active_bound.saturating_mul(2).min(MAX_BOUND);
        }
    }

    fn seed_proofs_from(&mut self, forcing: &MateSearch) {
        debug_assert_eq!(self.target, forcing.target);
        for (&key, &entry) in &forcing.table {
            if entry.proof == 0 {
                self.table.insert(
                    SearchKey {
                        quiet_left: 0,
                        ..key
                    },
                    entry,
                );
            }
        }
        if let Some(bound) = forcing.exact {
            self.active_bound = bound;
        }
    }

    fn entry(&mut self, key: SearchKey, moves: &mut MoveCache) -> Entry {
        if let Some(entry) = self.table.get(&key) {
            return *entry;
        }
        let entry = if let Some(winner) = key.position.captured_snipe_winner() {
            terminal_entry(winner == self.target)
        } else if key.position.leading == 0 && !has_legal_action(key.position) {
            terminal_entry(key.position.active_player().opponent() == self.target)
        } else if key.position.leading == 0 && key.plies_left == 0 {
            terminal_entry(false)
        } else if key.position.leading == 0
            && key.plies_left == 1
            && key.position.active_player() != self.target
        {
            // Resolve only the exceptional ways the mover can lose during its
            // own ply; do not expand a full irrelevant defender subtree.
            let resolution = moves.resolve_opponent_ply(key.position);
            let mut entry = terminal_entry(resolution.target_wins);
            entry.best = Some(resolution.best);
            entry
        } else if let Some(line) = moves.capture_mate(key.position) {
            if key.position.active_player() == self.target {
                Entry {
                    proof: 0,
                    disproof: INF,
                    best: Some(line),
                }
            } else {
                Entry {
                    proof: INF,
                    disproof: 0,
                    best: Some(line),
                }
            }
        } else {
            Entry::UNKNOWN
        };
        self.table.insert(key, entry);
        entry
    }

    fn dfpn(
        &mut self,
        key: SearchKey,
        threshold_proof: u64,
        threshold_disproof: u64,
        moves: &mut MoveCache,
        budget: &mut usize,
    ) {
        if *budget == 0 {
            return;
        }
        *budget -= 1;

        let initial = self.entry(key, moves);
        if initial.proof == 0 || initial.disproof == 0 {
            return;
        }

        loop {
            if *budget == 0 {
                return;
            }
            *budget -= 1;

            let is_or = key.position.active_player() == self.target;
            let complete_turn =
                key.position.leading == 0 && moves.can_expand_complete_turn(key.position);
            let forcing_node = self.forcing_only && is_or && key.position.leading == 0;
            let can_expand_forcing = !forcing_node || moves.can_expand_forcing_turn(key.position);
            let forcing_choices = forcing_node.then(|| moves.forcing_choices(key.position));
            let choices = if let Some(forcing) = &forcing_choices
                && (!can_expand_forcing || key.quiet_left == 0)
            {
                Arc::clone(forcing)
            } else if forcing_choices.is_some() {
                moves.turn_choices(key.position)
            } else if complete_turn {
                moves.evasion_choices(key.position)
            } else {
                moves.search_choices(key.position, complete_turn)
            };
            if choices.is_empty() {
                let winner = key.position.active_player().opponent();
                self.table
                    .insert(key, terminal_entry(winner == self.target));
                return;
            }

            let mut proof = if is_or { INF } else { 0 };
            let mut disproof = if is_or { 0 } else { INF };
            let mut best = None;
            let mut best_value = INF;

            for choice in choices.iter() {
                let child = child_key(
                    key,
                    *choice,
                    next_quiet_left(key, *choice, forcing_choices.as_deref()),
                );
                let child_entry = self.entry(child, moves);
                if is_or {
                    if child_entry.proof < best_value {
                        best_value = child_entry.proof;
                        best = Some(choice.line);
                    }
                    proof = proof.min(child_entry.proof);
                    disproof = capped_add(disproof, child_entry.disproof);
                } else {
                    if child_entry.disproof < best_value {
                        best_value = child_entry.disproof;
                        best = Some(choice.line);
                    }
                    proof = capped_add(proof, child_entry.proof);
                    disproof = disproof.min(child_entry.disproof);
                }
            }

            let aggregate = Entry {
                proof,
                disproof,
                best,
            };
            self.table.insert(key, aggregate);
            if proof >= threshold_proof
                || disproof >= threshold_disproof
                || proof == 0
                || disproof == 0
            {
                return;
            }

            let selected_line = best.expect("a non-terminal node has a most-proving child");
            let selected_choice = *choices
                .iter()
                .find(|candidate| candidate.line == selected_line)
                .expect("the most-proving choice came from this node");
            let selected = child_key(
                key,
                selected_choice,
                next_quiet_left(key, selected_choice, forcing_choices.as_deref()),
            );
            let selected_entry = self.entry(selected, moves);
            let mut second = INF;
            for choice in choices.iter() {
                if choice.line == selected_line {
                    continue;
                }
                let candidate = self.entry(
                    child_key(
                        key,
                        *choice,
                        next_quiet_left(key, *choice, forcing_choices.as_deref()),
                    ),
                    moves,
                );
                let value = if is_or {
                    candidate.proof
                } else {
                    candidate.disproof
                };
                second = second.min(value);
            }

            let (child_threshold_proof, child_threshold_disproof) = if is_or {
                (
                    threshold_proof.min(second.saturating_add(1).min(INF)),
                    threshold_disproof
                        .saturating_sub(disproof)
                        .saturating_add(selected_entry.disproof)
                        .min(INF),
                )
            } else {
                (
                    threshold_proof
                        .saturating_sub(proof)
                        .saturating_add(selected_entry.proof)
                        .min(INF),
                    threshold_disproof.min(second.saturating_add(1).min(INF)),
                )
            };
            self.dfpn(
                selected,
                child_threshold_proof,
                child_threshold_disproof,
                moves,
                budget,
            );
        }
    }

    fn exact_line(
        &self,
        root: Position,
        distance: u8,
        moves: &mut MoveCache,
    ) -> Result<Vec<Action>, String> {
        let mut position = root;
        let mut upper = distance;
        let mut lower = distance.saturating_sub(1);
        let mut line = Vec::with_capacity(usize::from(distance) * 2);

        for _ in 0..(usize::from(distance) * 2 + 2) {
            if position.captured_snipe_winner().is_some() {
                return Ok(line);
            }
            let is_or = position.active_player() == self.target;
            if is_or && let Some(mate) = moves.capture_mate(position) {
                line.push(mate.first);
                if let Some(second) = mate.second {
                    line.push(second);
                }
                return Ok(line);
            }
            let complete_turn = position.leading == 0 && moves.can_expand_complete_turn(position);
            let choices = moves.search_choices(position, complete_turn);
            if choices.is_empty() {
                return Ok(line);
            }
            let upper_key = SearchKey {
                position,
                plies_left: upper,
                quiet_left: 0,
            };
            let lower_key = SearchKey {
                position,
                plies_left: lower,
                quiet_left: 0,
            };
            let preferred = if is_or {
                self.table.get(&upper_key).and_then(|entry| entry.best)
            } else {
                self.table
                    .get(&lower_key)
                    .and_then(|entry| entry.best)
                    .or_else(|| self.table.get(&upper_key).and_then(|entry| entry.best))
            };
            let is_optimal = |candidate: &SearchChoice| {
                let upper_child = child_key(upper_key, *candidate, 0);
                let proves_upper = self
                    .table
                    .get(&upper_child)
                    .is_some_and(|entry| entry.proof == 0);
                if is_or {
                    return proves_upper;
                }
                if lower == 1 && position.active_player() != self.target {
                    return proves_upper
                        && self.table.get(&lower_key).and_then(|entry| entry.best)
                            == Some(candidate.line);
                }
                let lower_child = child_key(lower_key, *candidate, 0);
                proves_upper
                    && self
                        .table
                        .get(&lower_child)
                        .is_some_and(|entry| entry.disproof == 0)
            };
            let choice_line = preferred
                .filter(|preferred| {
                    choices
                        .iter()
                        .find(|candidate| candidate.line == *preferred)
                        .is_some_and(is_optimal)
                })
                .or_else(|| {
                    choices
                        .iter()
                        .find(|candidate| is_optimal(candidate))
                        .map(|candidate| candidate.line)
                })
                .ok_or_else(|| {
                    let upper_proofs = choices
                        .iter()
                        .filter(|candidate| {
                            self.table
                                .get(&child_key(upper_key, **candidate, 0))
                                .is_some_and(|entry| entry.proof == 0)
                        })
                        .count();
                    let lower_disproofs = choices
                        .iter()
                        .filter(|candidate| {
                            self.table
                                .get(&child_key(lower_key, **candidate, 0))
                                .is_some_and(|entry| entry.disproof == 0)
                        })
                        .count();
                    format!(
                        "no optimal continuation at upper {upper}, lower {lower}, or={is_or}, choices={}, upper-proofs={upper_proofs}, lower-disproofs={lower_disproofs}",
                        choices.len()
                    )
                })?;
            let choice = if let Some(choice) = choices
                .iter()
                .find(|candidate| candidate.line == choice_line)
                .copied()
            {
                choice
            } else {
                moves
                    .turn_choices(position)
                    .iter()
                    .find(|candidate| candidate.line == choice_line)
                    .copied()
                    .ok_or_else(|| "stored continuation is not legal".to_owned())?
            };
            if choice.consumes_ply {
                upper = upper.saturating_sub(1);
                lower = lower.saturating_sub(1);
            }
            line.push(choice.line.first);
            if let Some(second) = choice.line.second {
                line.push(second);
            }
            position = choice.next;
        }
        Err("principal variation exceeded its mate bound".to_owned())
    }
}

fn next_quiet_left(
    parent: SearchKey,
    choice: SearchChoice,
    forcing: Option<&[SearchChoice]>,
) -> u8 {
    let Some(forcing) = forcing else {
        return parent.quiet_left;
    };
    if forcing
        .iter()
        .any(|candidate| candidate.line == choice.line)
    {
        FORCING_QUIET_PLIES
    } else {
        parent.quiet_left.saturating_sub(1)
    }
}

fn child_key(parent: SearchKey, choice: SearchChoice, quiet_left: u8) -> SearchKey {
    SearchKey {
        position: choice.next,
        plies_left: if choice.consumes_ply {
            parent.plies_left.saturating_sub(1)
        } else {
            parent.plies_left
        },
        quiet_left,
    }
}

const fn terminal_entry(success: bool) -> Entry {
    if success {
        Entry {
            proof: 0,
            disproof: INF,
            best: None,
        }
    } else {
        Entry {
            proof: INF,
            disproof: 0,
            best: None,
        }
    }
}

fn capped_add(left: u64, right: u64) -> u64 {
    left.saturating_add(right).min(INF)
}

/// A shortest-forced-mate specialist.  Before a proof is complete Honey
/// reports a neutral estimate and a legal, mate-ordered fallback turn.
pub struct HoneyAnalyzer {
    root_state: Option<State>,
    root: Option<Position>,
    searches: Option<[MateSearch; 2]>,
    forcing_searches: Option<[MateSearch; 2]>,
    forcing_seeded: [bool; 2],
    moves: MoveCache,
    next_search: usize,
    solved: Option<OptimalOutcome>,
    line: Vec<Action>,
    line_error: Option<String>,
}

impl HoneyAnalyzer {
    pub fn new() -> Self {
        Self {
            root_state: None,
            root: None,
            searches: None,
            forcing_searches: None,
            forcing_seeded: [false; 2],
            moves: MoveCache::default(),
            next_search: 0,
            solved: None,
            line: Vec::new(),
            line_error: None,
        }
    }

    /// Compact profiler/benchmark state; this is intentionally diagnostic and
    /// does not participate in move selection.
    pub fn diagnostics(&self) -> String {
        fn describe(search: &MateSearch) -> String {
            format!(
                "{:?}:lo{} hi{:?} at{} exact{:?} tt{}",
                search.target,
                search.lower,
                search.upper,
                search.active_bound,
                search.exact,
                search.table.len()
            )
        }
        let full = self
            .searches
            .as_ref()
            .map(|searches| format!("{}; {}", describe(&searches[0]), describe(&searches[1])))
            .unwrap_or_else(|| "terminal".to_owned());
        let forcing = self
            .forcing_searches
            .as_ref()
            .map(|searches| format!("{}; {}", describe(&searches[0]), describe(&searches[1])))
            .unwrap_or_else(|| "terminal".to_owned());
        format!(
            "full[{full}] force[{forcing}] actions={} action_nodes={} turns={} evasions={} threats={} capture_tests={} horizons={} retained={} line_error={:?}",
            self.moves.actions.len(),
            self.moves.action_choices.len(),
            self.moves.complete_turns.len(),
            self.moves.evasion_turns.len(),
            self.moves.forcing_turns.len(),
            self.moves.capture_mates.len(),
            self.moves.one_ply_resolutions.len(),
            self.retained_entries(),
            self.line_error,
        )
    }

    fn retained_entries(&self) -> usize {
        let search_entries = self
            .searches
            .iter()
            .flatten()
            .chain(self.forcing_searches.iter().flatten())
            .map(|search| search.table.len())
            .sum::<usize>();
        search_entries
            + self.moves.actions.len()
            + self.moves.action_choices.len()
            + self.moves.complete_turns.len()
            + self.moves.evasion_turns.len()
            + self.moves.forcing_turns.len()
            + self.moves.capture_mates.len()
            + self.moves.one_ply_resolutions.len()
    }

    fn finish_if_solved(&mut self) {
        if self.solved.is_some() {
            return;
        }
        let Some(root) = self.root else {
            return;
        };
        let Some(searches) = self.searches.as_ref() else {
            return;
        };
        let solved_index = searches.iter().position(|search| search.exact.is_some());
        let Some(index) = solved_index else {
            return;
        };
        let target = searches[index].target;
        let distance = searches[index].exact.expect("selected exact search");
        let line = match searches[index].exact_line(root, distance, &mut self.moves) {
            Ok(line) => line,
            Err(error) => {
                self.line_error = Some(format!(
                    "could not extract target {target:?} distance {distance}: {error}"
                ));
                return;
            }
        };
        if !self.validate_line(&line, target, u32::from(distance)) {
            self.line_error = Some(self.describe_line_failure(&line, target, u32::from(distance)));
            return;
        }
        let mate = MateInN::new(target, u32::from(distance)).expect("Honey bounds fit MateInN");
        self.line = line;
        self.solved = Some(OptimalOutcome::MateInN(mate));
        self.line_error = None;
    }

    fn validate_line(&self, line: &[Action], target: Player, expected_plies: u32) -> bool {
        let Some(mut state) = self.root_state.clone() else {
            return false;
        };
        let mut plies = 0;
        for &action in line {
            if state.leading_action.is_none() {
                plies += 1;
            }
            let Ok(next) = state.apply(action) else {
                return false;
            };
            state = next;
            if state.winner().is_some() {
                break;
            }
        }
        state.winner() == Some(target) && plies == expected_plies
    }

    fn describe_line_failure(
        &self,
        line: &[Action],
        target: Player,
        expected_plies: u32,
    ) -> String {
        let Some(mut state) = self.root_state.clone() else {
            return "missing root state".to_owned();
        };
        let mut plies = 0;
        for (index, &action) in line.iter().enumerate() {
            if state.leading_action.is_none() {
                plies += 1;
            }
            match state.apply(action) {
                Ok(next) => state = next,
                Err(error) => return format!("illegal action {index}: {error:?}"),
            }
            if state.winner().is_some() {
                break;
            }
        }
        format!(
            "expected {target:?} in {expected_plies}, got {:?} in {plies} from {} actions: {line:?}",
            state.winner(),
            line.len()
        )
    }

    fn fallback_line(&mut self, state: &State, root: Position) -> Vec<Action> {
        let mut line = Vec::with_capacity(2);
        let mut position = root;
        let actions = self.moves.actions(position);
        let Some(first) = actions.first() else {
            return line;
        };
        line.push(first.action);
        position = position.apply_known(first.action, first.capture);
        if position.active_player() == root.active_player()
            && position.captured_snipe_winner().is_none()
        {
            let second = self.moves.actions(position);
            if let Some(action) = second.first() {
                line.push(action.action);
            }
        }
        debug_assert!(
            line.iter()
                .try_fold(state.clone(), |current, &action| current.apply(action))
                .is_ok()
        );
        line
    }
}

impl Default for HoneyAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for HoneyAnalyzer {
    fn set_state(&mut self, state: State) {
        self.moves = MoveCache::default();
        self.next_search = 0;
        self.forcing_seeded = [false; 2];
        self.root_state = Some(state.clone());
        self.line_error = None;

        if let Some(winner) = state.winner() {
            let mate = MateInN::new(winner, 0).expect("mate zero is representable");
            self.solved = Some(OptimalOutcome::MateInN(mate));
            self.root = None;
            self.searches = None;
            self.forcing_searches = None;
            self.line.clear();
        } else {
            let root = Position::from_core(&state);
            self.root = Some(root);
            self.line = self.fallback_line(&state, root);
            self.solved = None;
            let active = state.active_player;
            self.searches = Some([
                MateSearch::new(active, false),
                MateSearch::new(active.opponent(), false),
            ]);
            self.forcing_searches = Some([
                MateSearch::new(active, true),
                MateSearch::new(active.opponent(), true),
            ]);
        }
    }

    fn think_for_one_tick(&mut self) {
        if self.solved.is_some() {
            return;
        }
        let Some(root) = self.root else {
            return;
        };
        let candidate = self
            .searches
            .as_ref()
            .and_then(|searches| searches.iter().position(|search| search.upper.is_some()));
        if let Some(index) = candidate {
            self.forcing_searches = None;
            let Some(searches) = self.searches.as_mut() else {
                return;
            };
            searches[1 - index].table.clear();
            searches[index].think(root, &mut self.moves, WORK_PER_TICK);
            self.finish_if_solved();
            return;
        }
        if let Some(index) = self.forcing_seeded.iter().position(|&seeded| seeded) {
            let Some(searches) = self.searches.as_mut() else {
                return;
            };
            searches[index].think(root, &mut self.moves, WORK_PER_TICK);
            self.finish_if_solved();
            return;
        }

        let lane = self.next_search;
        self.next_search = (self.next_search + 1) % 8;
        if lane < 6 {
            let index = lane % 2;
            let Some(forcing) = self.forcing_searches.as_mut() else {
                return;
            };
            forcing[index].think(root, &mut self.moves, WORK_PER_TICK);
            if forcing[index].exact.is_some() && !self.forcing_seeded[index] {
                let Some(searches) = self.searches.as_mut() else {
                    return;
                };
                searches[index].seed_proofs_from(&forcing[index]);
                self.forcing_seeded[index] = true;
            }
        } else {
            let index = lane - 6;
            let Some(searches) = self.searches.as_mut() else {
                return;
            };
            searches[index].think(root, &mut self.moves, WORK_PER_TICK);
        }
        self.finish_if_solved();
    }

    fn is_fully_solved(&self) -> Option<OptimalOutcome> {
        self.solved
    }

    fn evaluation(&self) -> Evaluation {
        self.solved
            .map(OptimalOutcome::as_evaluation)
            .unwrap_or(EvaluationEstimate::ZERO.into())
    }

    fn write_optimal_lop<W>(&self, writer: &mut W)
    where
        W: ActionWriter,
    {
        writer.reserve(self.line.len());
        for &action in &self.line {
            writer.push(action);
        }
    }
}

fn activates_triplet(actor: usize, destination: u16) -> bool {
    let actor_bit = bit(actor as u8);
    let animals = destination | actor_bit;
    for element in 0..4 {
        if actor_bit & (UNARY[element] | BINARY[element] | TERNARY[element]) != 0
            && animals & UNARY[element] != 0
            && animals & BINARY[element] != 0
            && animals & TERNARY[element] != 0
        {
            return true;
        }
    }
    false
}

fn token(player: Player, location: u8) -> u8 {
    location | (player_bit(player) << 3)
}

fn token_location(value: u8) -> u8 {
    value & 0b111
}

fn token_player(value: u8) -> Player {
    if value & 0b1000 == 0 {
        Player::Alpha
    } else {
        Player::Beta
    }
}

fn player_bit(player: Player) -> u8 {
    u8::from(player == Player::Beta)
}

fn animal_index(animal: Animal) -> usize {
    animal as usize
}

fn encode_leading(leading: Option<AnimalStep>) -> u8 {
    let Some(step) = leading else {
        return 0;
    };
    1 | ((animal_index(step.actor) as u8) << 1) | ((rank_number(step.destination) - 1) << 5)
}

fn decode_leading(encoded: u8) -> Option<AnimalStep> {
    if encoded == 0 {
        return None;
    }
    let actor = ANIMALS[((encoded >> 1) & 0x0f) as usize];
    let destination = number_rank(((encoded >> 5) & 0x07) + 1);
    Some(AnimalStep {
        actor,
        direction: StepDirection::Advance,
        destination,
    })
}

fn advance(player: Player, source: u8) -> Option<u8> {
    match player {
        Player::Alpha if source < 6 => Some(source + 1),
        Player::Beta if source > 1 => Some(source - 1),
        _ => None,
    }
}

fn retreat(player: Player, source: u8) -> Option<u8> {
    advance(player.opponent(), source)
}

fn step_destination(player: Player, direction: StepDirection, source: u8) -> Option<u8> {
    match direction {
        StepDirection::Advance => advance(player, source),
        StepDirection::Retreat => retreat(player, source),
    }
}

fn step_source(player: Player, direction: StepDirection, destination: u8) -> Option<u8> {
    match direction {
        StepDirection::Advance => retreat(player, destination),
        StepDirection::Retreat => advance(player, destination),
    }
}

fn legal_retreater_drop(player: Player, destination: u8) -> bool {
    match player {
        Player::Alpha => destination <= 4,
        Player::Beta => destination >= 3,
    }
}

fn rank_number(rank: Rank) -> u8 {
    match rank {
        Rank::R1 => 1,
        Rank::R2 => 2,
        Rank::R3 => 3,
        Rank::R4 => 4,
        Rank::R5 => 5,
        Rank::R6 => 6,
    }
}

fn number_rank(number: u8) -> Rank {
    match number {
        1 => Rank::R1,
        2 => Rank::R2,
        3 => Rank::R3,
        4 => Rank::R4,
        5 => Rank::R5,
        6 => Rank::R6,
        _ => unreachable!("rank is in 1..=6"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use snipe_core::{CardMultiset, InitialStateBuilder};
    use snipe_prng::initial_state;
    use std::collections::HashSet;

    fn cards(entries: &[(Card, Player)]) -> CardMultiset {
        let mut cards = CardMultiset::EMPTY;
        for &(card, player) in entries {
            cards = cards
                .checked_add(CardMultiset::singleton(card, player))
                .unwrap();
        }
        cards
    }

    fn immediate_mate() -> State {
        State {
            active_player: Player::Alpha,
            reserves: cards(&[
                (Card::Animal(Animal::Ox), Player::Alpha),
                (Card::Animal(Animal::Ox), Player::Beta),
                (Card::Animal(Animal::Rabbit), Player::Alpha),
                (Card::Animal(Animal::Rabbit), Player::Beta),
                (Card::Animal(Animal::Dragon), Player::Alpha),
                (Card::Animal(Animal::Dragon), Player::Beta),
                (Card::Animal(Animal::Snake), Player::Alpha),
                (Card::Animal(Animal::Snake), Player::Beta),
                (Card::Animal(Animal::Horse), Player::Alpha),
                (Card::Animal(Animal::Horse), Player::Beta),
                (Card::Animal(Animal::Ram), Player::Alpha),
                (Card::Animal(Animal::Ram), Player::Beta),
                (Card::Animal(Animal::Monkey), Player::Alpha),
                (Card::Animal(Animal::Monkey), Player::Beta),
                (Card::Animal(Animal::Dog), Player::Alpha),
                (Card::Animal(Animal::Dog), Player::Beta),
                (Card::Animal(Animal::Boar), Player::Alpha),
                (Card::Animal(Animal::Boar), Player::Beta),
                (Card::Animal(Animal::Fish), Player::Alpha),
                (Card::Animal(Animal::Fish), Player::Beta),
                (Card::Animal(Animal::Elephant), Player::Alpha),
                (Card::Animal(Animal::Elephant), Player::Beta),
                (Card::Animal(Animal::Squid), Player::Alpha),
                (Card::Animal(Animal::Squid), Player::Beta),
                (Card::Animal(Animal::Frog), Player::Alpha),
                (Card::Animal(Animal::Frog), Player::Beta),
            ]),
            r1: cards(&[
                (Card::Snipe, Player::Alpha),
                (Card::Animal(Animal::Mouse), Player::Alpha),
            ]),
            r2: cards(&[
                (Card::Animal(Animal::Rooster), Player::Beta),
                (Card::Animal(Animal::Tiger), Player::Beta),
                (Card::Snipe, Player::Beta),
            ]),
            r3: CardMultiset::EMPTY,
            r4: CardMultiset::EMPTY,
            r5: CardMultiset::EMPTY,
            r6: cards(&[
                (Card::Animal(Animal::Mouse), Player::Beta),
                (Card::Animal(Animal::Rooster), Player::Alpha),
                (Card::Animal(Animal::Tiger), Player::Alpha),
            ]),
            leading_action: None,
        }
    }

    #[test]
    fn packed_rules_match_core_through_reachable_play() {
        for seed in 0..8 {
            let mut state = initial_state(seed);
            for ply in 0..24 {
                let position = Position::from_core(&state);
                let honey_actions = generate_actions(position)
                    .into_iter()
                    .map(|candidate| candidate.action)
                    .collect::<HashSet<_>>();
                let mut core_actions = Vec::new();
                state.write_legal_actions(&mut core_actions);
                assert_eq!(
                    has_legal_action(position),
                    !core_actions.is_empty(),
                    "seed {seed}, action {ply}: cheap legal-action test"
                );
                let core_actions = core_actions.into_iter().collect::<HashSet<_>>();
                assert_eq!(honey_actions, core_actions, "seed {seed}, action {ply}");
                let Some(&action) = core_actions
                    .iter()
                    .nth((seed as usize + ply) % core_actions.len().max(1))
                else {
                    break;
                };
                let core_next = state.clone().apply(action).unwrap();
                let packed_next = position.apply(action);
                assert_eq!(packed_next, Position::from_core(&core_next));
                state = core_next;
                if state.winner().is_some() {
                    break;
                }
            }
        }
    }

    #[test]
    fn every_pruned_defense_has_a_checked_mate_certificate() {
        let mut omitted = 0;
        for seed in 0..32 {
            let mut state = initial_state(seed);
            for ply in 0..40 {
                let position = Position::from_core(&state);
                let mut moves = MoveCache::default();
                if position.leading == 0 && moves.can_expand_complete_turn(position) {
                    let all = moves.turn_choices(position);
                    let evasions = moves.evasion_choices(position);
                    for choice in all.iter().copied() {
                        if evasions.iter().any(|evasion| evasion.next == choice.next) {
                            continue;
                        }
                        omitted += 1;
                        let attacker = position.active_player().opponent();
                        if choice.next.captured_snipe_winner() == Some(attacker)
                            || !has_legal_action(choice.next)
                        {
                            continue;
                        }
                        let certificate = moves
                            .capture_mate(choice.next)
                            .expect("an omitted defense leaves an explicit capture-mate");
                        let after_first = choice.next.apply(certificate.first);
                        let after_mate = certificate
                            .second
                            .map_or(after_first, |second| after_first.apply(second));
                        assert_eq!(after_mate.captured_snipe_winner(), Some(attacker));
                    }
                }

                let mut actions = Vec::new();
                state.write_legal_actions(&mut actions);
                let Some(&action) = actions.get((seed as usize + ply) % actions.len().max(1))
                else {
                    break;
                };
                state = state.apply(action).unwrap();
                if state.winner().is_some() {
                    break;
                }
            }
        }
        assert!(omitted > 0, "the corpus must exercise the evasion filter");
    }

    #[test]
    fn one_ply_horizon_matches_exhaustive_completed_turns() {
        for seed in 0..12 {
            let mut state = initial_state(seed);
            for ply in 0..28 {
                if state.leading_action.is_none() {
                    let position = Position::from_core(&state);
                    let mut moves = MoveCache::default();
                    let turns = moves.turn_choices(position);
                    if !turns.is_empty() {
                        let target = position.active_player().opponent();
                        let forced_loss = turns.iter().all(|choice| {
                            choice.next.captured_snipe_winner().or_else(|| {
                                (!has_legal_action(choice.next))
                                    .then(|| choice.next.active_player().opponent())
                            }) == Some(target)
                        });
                        let resolution = moves.resolve_opponent_ply(position);
                        assert_eq!(
                            resolution.target_wins, forced_loss,
                            "seed {seed}, ply {ply}"
                        );
                        assert!(turns.iter().any(|choice| choice.line == resolution.best));
                    }
                }

                let mut actions = Vec::new();
                state.write_legal_actions(&mut actions);
                let Some(&action) = actions.get((seed as usize + ply) % actions.len().max(1))
                else {
                    break;
                };
                state = state.apply(action).unwrap();
                if state.winner().is_some() {
                    break;
                }
            }
        }
    }

    #[test]
    fn proves_and_reports_the_shortest_immediate_mate() {
        let state = immediate_mate();
        let mut honey = HoneyAnalyzer::new();
        honey.set_state(state.clone());
        for _ in 0..2_000 {
            if honey.is_fully_solved().is_some() {
                break;
            }
            honey.think_for_one_tick();
        }
        let expected = MateInN::new(Player::Alpha, 1).unwrap();
        assert_eq!(
            honey.is_fully_solved(),
            Some(OptimalOutcome::MateInN(expected))
        );
        assert_eq!(honey.evaluation(), Evaluation::MateInN(expected));
        let mut line = Vec::new();
        honey.write_optimal_lop(&mut line);
        assert!(!line.is_empty());
        let mut after = state;
        for action in line {
            after = after.apply(action).unwrap();
        }
        assert_eq!(after.winner(), Some(Player::Alpha));
    }

    #[test]
    fn recognizes_terminal_positions_without_thinking() {
        let mut state = immediate_mate();
        state = state
            .apply(Action::AnimalStep(AnimalStep {
                actor: Animal::Mouse,
                direction: StepDirection::Advance,
                destination: Rank::R2,
            }))
            .unwrap();
        let mut honey = HoneyAnalyzer::new();
        honey.set_state(state);
        let mate = MateInN::new(Player::Alpha, 0).unwrap();
        assert_eq!(honey.is_fully_solved(), Some(OptimalOutcome::MateInN(mate)));
        assert_eq!(honey.evaluation(), Evaluation::MateInN(mate));
    }

    #[test]
    fn double_snipe_capture_is_not_a_false_beta_mate_certificate() {
        let mut state = immediate_mate();
        state.active_player = Player::Beta;
        state.r1 = state.r1.remove_one(Card::Snipe, Player::Alpha).unwrap();
        state.r2 = state
            .r2
            .checked_add(CardMultiset::singleton(Card::Snipe, Player::Alpha))
            .unwrap();
        state.r6 = state
            .r6
            .remove_one(Card::Animal(Animal::Mouse), Player::Beta)
            .unwrap();
        state.r3 = CardMultiset::singleton(Card::Animal(Animal::Mouse), Player::Beta);
        let capture = Action::AnimalStep(AnimalStep {
            actor: Animal::Mouse,
            direction: StepDirection::Advance,
            destination: Rank::R2,
        });
        assert_eq!(
            state.clone().apply(capture).unwrap().winner(),
            Some(Player::Alpha)
        );

        let position = Position::from_core(&state);
        assert_eq!(position.winning_capture(), None);
        let mut moves = MoveCache::default();
        assert_eq!(moves.capture_mate(position), None);
    }

    #[test]
    fn unsolved_positions_have_no_material_heuristic() {
        let mut honey = HoneyAnalyzer::new();
        honey.set_state(initial_state(7));
        assert_eq!(honey.evaluation(), EvaluationEstimate::ZERO.into());
        let mut line = Vec::new();
        honey.write_optimal_lop(&mut line);
        assert!(!line.is_empty());
    }

    #[test]
    fn initial_builder_still_round_trips_all_animals() {
        let state = InitialStateBuilder {
            alpha_reserve: [Animal::Mouse],
            r1: [Animal::Ox, Animal::Tiger],
            r2: [
                Animal::Rabbit,
                Animal::Dragon,
                Animal::Snake,
                Animal::Horse,
                Animal::Ram,
                Animal::Monkey,
                Animal::Rooster,
                Animal::Dog,
                Animal::Boar,
                Animal::Fish,
                Animal::Elephant,
                Animal::Squid,
            ],
            r3: [Animal::Frog],
            r4: [Animal::Mouse],
            r5: [
                Animal::Ox,
                Animal::Tiger,
                Animal::Rabbit,
                Animal::Dragon,
                Animal::Snake,
                Animal::Horse,
                Animal::Ram,
                Animal::Monkey,
                Animal::Rooster,
                Animal::Dog,
                Animal::Boar,
                Animal::Fish,
            ],
            r6: [Animal::Elephant, Animal::Squid],
            beta_reserve: [Animal::Frog],
        }
        .build()
        .unwrap();
        let position = Position::from_core(&state);
        assert_eq!(position.active_player(), Player::Beta);
        assert_eq!(generate_actions(position).len(), {
            let mut actions = Vec::new();
            state.write_legal_actions(&mut actions);
            actions.len()
        });
    }
}
