//! Avocado: a deterministic, allocation-conscious Snipe Hunt searcher.
//!
//! Search work is split into fixed node batches. An interrupted iteration keeps
//! its completed transpositions, but it never publishes a partial result.
//! Consequently, every reported mate comes from a fully completed proof.

use snipe_core::{
    Action, ActionWriter, Analyzer, Animal, AnimalDrop, AnimalStep, Card, Evaluation,
    EvaluationEstimate, MateInN, Player, Rank, SnipeStep, State, StepDirection,
};
use std::{
    cmp::Reverse,
    collections::{HashMap, hash_map::Entry},
    hash::{BuildHasherDefault, Hash, Hasher},
};

const ACTION_CAPACITY: usize = 320;
const INITIAL_MOVE_CAPACITY: usize = 192;
const NODES_PER_TICK: u32 = 4_096;
const MATE_SCORE: i32 = 2_000_000;
const MATE_FLOOR: i32 = 1_000_000;
const INFINITY: i32 = MATE_SCORE + 1;

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

const EMPTY_ACTION: Action = Action::SnipeStep(snipe_core::SnipeStep {
    destination: Rank::R1,
});

const UNARY_MASKS: [u16; 4] = [
    animal_mask(&[Animal::Rooster, Animal::Boar, Animal::Squid]),
    animal_mask(&[Animal::Ox, Animal::Rabbit, Animal::Dog]),
    animal_mask(&[Animal::Mouse, Animal::Snake, Animal::Monkey]),
    animal_mask(&[Animal::Horse, Animal::Ram, Animal::Frog]),
];
const BINARY_MASKS: [u16; 4] = [
    animal_mask(&[Animal::Mouse, Animal::Horse, Animal::Dog]),
    animal_mask(&[Animal::Snake, Animal::Squid, Animal::Frog]),
    animal_mask(&[Animal::Ox, Animal::Ram, Animal::Boar]),
    animal_mask(&[Animal::Rabbit, Animal::Monkey, Animal::Rooster]),
];
const TERNARY_MASKS: [u16; 4] = [
    animal_mask(&[Animal::Tiger]),
    animal_mask(&[Animal::Fish]),
    animal_mask(&[Animal::Elephant]),
    animal_mask(&[Animal::Dragon]),
];
const RETREATER_MASK: u16 = animal_mask(&[
    Animal::Mouse,
    Animal::Rabbit,
    Animal::Snake,
    Animal::Ram,
    Animal::Boar,
    Animal::Squid,
]);

const fn animal_mask(animals: &[Animal]) -> u16 {
    let mut result = 0;
    let mut index = 0;
    while index < animals.len() {
        result |= 1 << animals[index] as u16;
        index += 1;
    }
    result
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct Ply {
    first: Action,
    second: Option<Action>,
}

impl Ply {
    fn write<W: ActionWriter>(self, writer: &mut W) {
        writer.push(self.first);
        if let Some(second) = self.second {
            writer.push(second);
        }
    }
}

struct ActionBuffer {
    actions: [Action; ACTION_CAPACITY],
    len: usize,
}

impl ActionBuffer {
    fn new() -> Self {
        Self {
            actions: [EMPTY_ACTION; ACTION_CAPACITY],
            len: 0,
        }
    }

    fn clear(&mut self) {
        self.len = 0;
    }

    fn as_slice(&self) -> &[Action] {
        &self.actions[..self.len]
    }

    fn is_empty(&self) -> bool {
        self.len == 0
    }
}

impl ActionWriter for ActionBuffer {
    fn push(&mut self, action: Action) {
        assert!(
            self.len < self.actions.len(),
            "position generated more than {ACTION_CAPACITY} legal actions"
        );
        self.actions[self.len] = action;
        self.len += 1;
    }

    fn reserve(&mut self, additional: usize) {
        assert!(
            self.len.saturating_add(additional) <= self.actions.len(),
            "position requested space for more than {ACTION_CAPACITY} legal actions"
        );
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Pile {
    alpha: u16,
    beta: u16,
    twins: u16,
    snipes: u8,
}

impl Pile {
    const EMPTY: Self = Self {
        alpha: 0,
        beta: 0,
        twins: 0,
        snipes: 0,
    };

    fn from_core(cards: snipe_core::CardMultiset) -> Self {
        let mut result = Self::EMPTY;
        for animal in ANIMALS {
            for player in [Player::Alpha, Player::Beta] {
                let count = cards.count(Card::Animal(animal), player);
                if count != 0 {
                    result.set_presence(animal, player);
                }
                if count == 2 {
                    result.twins |= animal_bit(animal);
                }
            }
        }
        if cards.count(Card::Snipe, Player::Alpha) != 0 {
            result.snipes |= snipe_bit(Player::Alpha);
        }
        if cards.count(Card::Snipe, Player::Beta) != 0 {
            result.snipes |= snipe_bit(Player::Beta);
        }
        result
    }

    fn animal_count(self, animal: Animal, player: Player) -> u8 {
        let bit = animal_bit(animal);
        if self.presence(player) & bit == 0 {
            0
        } else if self.twins & bit != 0 {
            2
        } else {
            1
        }
    }

    fn animal_total(self, player: Player) -> u32 {
        self.presence(player).count_ones() + (self.twins & self.presence(player)).count_ones()
    }

    fn card_total(self) -> u32 {
        self.alpha.count_ones()
            + self.beta.count_ones()
            + self.twins.count_ones()
            + self.snipes.count_ones()
    }

    fn has_snipe(self, player: Player) -> bool {
        self.snipes & snipe_bit(player) != 0
    }

    fn add_animal(&mut self, animal: Animal, player: Player) {
        let bit = animal_bit(animal);
        let own = self.presence(player) & bit != 0;
        let enemy = self.presence(player.opponent()) & bit != 0;
        if own {
            debug_assert!(!enemy && self.twins & bit == 0);
            self.twins |= bit;
        } else {
            debug_assert!(self.twins & bit == 0);
            self.set_presence(animal, player);
        }
    }

    fn remove_animal(&mut self, animal: Animal, player: Player) {
        let bit = animal_bit(animal);
        debug_assert!(self.presence(player) & bit != 0);
        if self.twins & bit != 0 {
            self.twins &= !bit;
        } else {
            *self.presence_mut(player) &= !bit;
        }
    }

    fn add_snipe(&mut self, player: Player) {
        let bit = snipe_bit(player);
        debug_assert_eq!(self.snipes & bit, 0);
        self.snipes |= bit;
    }

    fn remove_snipe(&mut self, player: Player) {
        let bit = snipe_bit(player);
        debug_assert_ne!(self.snipes & bit, 0);
        self.snipes &= !bit;
    }

    fn presence(self, player: Player) -> u16 {
        match player {
            Player::Alpha => self.alpha,
            Player::Beta => self.beta,
        }
    }

    fn presence_mut(&mut self, player: Player) -> &mut u16 {
        match player {
            Player::Alpha => &mut self.alpha,
            Player::Beta => &mut self.beta,
        }
    }

    fn set_presence(&mut self, animal: Animal, player: Player) {
        *self.presence_mut(player) |= animal_bit(animal);
    }

    fn packed(self) -> u64 {
        u64::from(self.alpha)
            | (u64::from(self.beta) << 16)
            | (u64::from(self.twins) << 32)
            | (u64::from(self.snipes) << 48)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Position {
    locations: [Pile; 7],
    active_player: Player,
    leading_action: Option<AnimalStep>,
}

impl Hash for Position {
    fn hash<H: Hasher>(&self, state: &mut H) {
        for pile in self.locations {
            state.write_u64(pile.packed());
        }
        let mut metadata = u16::from(self.active_player == Player::Beta);
        if let Some(leading) = self.leading_action {
            metadata |= 1 << 1;
            metadata |= (leading.actor as u16) << 2;
            metadata |= u16::from(leading.direction == StepDirection::Retreat) << 6;
            metadata |= (leading.destination as u16) << 7;
        }
        state.write_u16(metadata);
    }
}

impl Position {
    fn from_core(state: &State) -> Self {
        Self {
            locations: [
                Pile::from_core(state.reserves),
                Pile::from_core(state.r1),
                Pile::from_core(state.r2),
                Pile::from_core(state.r3),
                Pile::from_core(state.r4),
                Pile::from_core(state.r5),
                Pile::from_core(state.r6),
            ],
            active_player: state.active_player,
            leading_action: state.leading_action,
        }
    }

    fn reserve(self) -> Pile {
        self.locations[0]
    }

    fn reserve_mut(&mut self) -> &mut Pile {
        &mut self.locations[0]
    }

    fn rank(self, rank: Rank) -> Pile {
        self.locations[rank as usize + 1]
    }

    fn rank_mut(&mut self, rank: Rank) -> &mut Pile {
        &mut self.locations[rank as usize + 1]
    }

    fn captured_snipe_winner(self) -> Option<Player> {
        if self.reserve().has_snipe(Player::Beta) {
            Some(Player::Alpha)
        } else if self.reserve().has_snipe(Player::Alpha) {
            Some(Player::Beta)
        } else {
            None
        }
    }

    fn snipe_rank(self, player: Player) -> Option<Rank> {
        RANKS
            .into_iter()
            .find(|&rank| self.rank(rank).has_snipe(player))
    }

    fn write_legal_actions<W: ActionWriter>(self, writer: &mut W) {
        if self.captured_snipe_winner().is_some() {
            return;
        }
        writer.reserve(290);
        if self.leading_action.is_none() {
            self.write_snipe_steps(writer);
            self.write_drops(writer);
        }
        self.write_animal_steps(writer);
    }

    fn write_snipe_steps<W: ActionWriter>(self, writer: &mut W) {
        let Some(source) = self.snipe_rank(self.active_player) else {
            return;
        };
        if self.rank(source).card_total() <= 1 {
            return;
        }
        if let Some(destination) = advance_destination(source, self.active_player) {
            writer.push(Action::SnipeStep(SnipeStep { destination }));
        }
        if let Some(destination) = retreat_destination(source, self.active_player) {
            writer.push(Action::SnipeStep(SnipeStep { destination }));
        }
    }

    fn write_drops<W: ActionWriter>(self, writer: &mut W) {
        if self.reserve().animal_total(self.active_player) <= 1 {
            return;
        }
        for animal in ANIMALS {
            if self.reserve().animal_count(animal, self.active_player) == 0 {
                continue;
            }
            for destination in RANKS {
                if !is_retreater(animal) || legal_retreater_drop(self.active_player, destination) {
                    writer.push(Action::Drop(AnimalDrop {
                        actor: animal,
                        destination,
                    }));
                }
            }
        }
    }

    fn write_animal_steps<W: ActionWriter>(self, writer: &mut W) {
        for source in RANKS {
            for animal in ANIMALS {
                if self.rank(source).animal_count(animal, self.active_player) == 0 {
                    continue;
                }
                if let Some(destination) = advance_destination(source, self.active_player) {
                    let step = AnimalStep {
                        actor: animal,
                        direction: StepDirection::Advance,
                        destination,
                    };
                    if self.validate_animal_step(step).is_some() {
                        writer.push(Action::AnimalStep(step));
                    }
                }
                if is_retreater(animal)
                    && let Some(destination) = retreat_destination(source, self.active_player)
                {
                    let step = AnimalStep {
                        actor: animal,
                        direction: StepDirection::Retreat,
                        destination,
                    };
                    if self.validate_animal_step(step).is_some() {
                        writer.push(Action::AnimalStep(step));
                    }
                }
            }
        }
    }

    fn validate_animal_step(self, step: AnimalStep) -> Option<Rank> {
        if step.direction == StepDirection::Retreat && !is_retreater(step.actor) {
            return None;
        }
        let source = source_for_destination(step.destination, self.active_player, step.direction)?;
        let friendly_count = self
            .rank(source)
            .animal_count(step.actor, self.active_player);
        if friendly_count == 0 {
            return None;
        }
        if let Some(leading) = self.leading_action
            && leading.actor == step.actor
            && leading.destination == source
            && friendly_count < 2
        {
            return None;
        }

        let destination = self.rank(step.destination);
        let activates = would_activate(step.actor, destination);
        let enemy_snipe = destination.has_snipe(self.active_player.opponent());
        let friendly_snipe = destination.has_snipe(self.active_player);
        if self.rank(source).card_total() <= 1 {
            if !activates || !enemy_snipe {
                return None;
            }
        } else if activates && friendly_snipe && !enemy_snipe {
            return None;
        }
        Some(source)
    }

    fn apply(mut self, action: Action) -> Option<Self> {
        if self.captured_snipe_winner().is_some() {
            return None;
        }
        let player = self.active_player;
        match action {
            Action::SnipeStep(step) => {
                if self.leading_action.is_some() {
                    return None;
                }
                let source = self.snipe_rank(player)?;
                let adjacent = advance_destination(source, player) == Some(step.destination)
                    || retreat_destination(source, player) == Some(step.destination);
                if !adjacent || self.rank(source).card_total() <= 1 {
                    return None;
                }
                self.rank_mut(source).remove_snipe(player);
                self.rank_mut(step.destination).add_snipe(player);
                self.active_player = player.opponent();
            }
            Action::Drop(drop) => {
                if self.leading_action.is_some()
                    || self.reserve().animal_count(drop.actor, player) == 0
                    || self.reserve().animal_total(player) <= 1
                    || (is_retreater(drop.actor) && !legal_retreater_drop(player, drop.destination))
                {
                    return None;
                }
                self.reserve_mut().remove_animal(drop.actor, player);
                self.rank_mut(drop.destination)
                    .add_animal(drop.actor, player);
                self.active_player = player.opponent();
            }
            Action::AnimalStep(step) => {
                let source = self.validate_animal_step(step)?;
                let destination_before = self.rank(step.destination);
                let activates = would_activate(step.actor, destination_before);
                self.rank_mut(source).remove_animal(step.actor, player);
                if activates {
                    self.capture_into_reserve(destination_before);
                    *self.rank_mut(step.destination) = Pile::EMPTY;
                    self.rank_mut(step.destination)
                        .add_animal(step.actor, player);
                } else {
                    self.rank_mut(step.destination)
                        .add_animal(step.actor, player);
                }
                if self.leading_action.is_some() {
                    self.leading_action = None;
                    self.active_player = player.opponent();
                } else {
                    self.leading_action = Some(step);
                }
            }
        }
        Some(self)
    }

    fn capture_into_reserve(&mut self, captured: Pile) {
        let capturer = self.active_player;
        for animal in ANIMALS {
            let count = captured.animal_count(animal, Player::Alpha)
                + captured.animal_count(animal, Player::Beta);
            for _ in 0..count {
                self.reserve_mut().add_animal(animal, capturer);
            }
        }
        for snipe in [Player::Alpha, Player::Beta] {
            if captured.has_snipe(snipe) {
                self.reserve_mut().add_snipe(snipe);
            }
        }
    }

    fn winner(self) -> Option<Player> {
        if let Some(winner) = self.captured_snipe_winner() {
            return Some(winner);
        }
        let mut actions = ActionBuffer::new();
        self.write_legal_actions(&mut actions);
        actions.is_empty().then_some(self.active_player.opponent())
    }
}

const RANKS: [Rank; 6] = [Rank::R1, Rank::R2, Rank::R3, Rank::R4, Rank::R5, Rank::R6];

const fn animal_bit(animal: Animal) -> u16 {
    1 << animal as u16
}

const fn snipe_bit(player: Player) -> u8 {
    match player {
        Player::Alpha => 1,
        Player::Beta => 2,
    }
}

fn is_retreater(animal: Animal) -> bool {
    RETREATER_MASK & animal_bit(animal) != 0
}

fn would_activate(actor: Animal, destination: Pile) -> bool {
    let present = destination.alpha | destination.beta;
    let actor = animal_bit(actor);
    for element in 0..4 {
        let actor_roles = u8::from(UNARY_MASKS[element] & actor != 0)
            | (u8::from(BINARY_MASKS[element] & actor != 0) << 1)
            | (u8::from(TERNARY_MASKS[element] & actor != 0) << 2);
        if actor_roles == 0 {
            continue;
        }
        let destination_roles = u8::from(UNARY_MASKS[element] & present != 0)
            | (u8::from(BINARY_MASKS[element] & present != 0) << 1)
            | (u8::from(TERNARY_MASKS[element] & present != 0) << 2);
        if actor_roles | destination_roles == 0b111 {
            return true;
        }
    }
    false
}

const fn advance_destination(rank: Rank, player: Player) -> Option<Rank> {
    match player {
        Player::Alpha => next_rank(rank),
        Player::Beta => previous_rank(rank),
    }
}

const fn retreat_destination(rank: Rank, player: Player) -> Option<Rank> {
    match player {
        Player::Alpha => previous_rank(rank),
        Player::Beta => next_rank(rank),
    }
}

const fn source_for_destination(
    destination: Rank,
    player: Player,
    direction: StepDirection,
) -> Option<Rank> {
    match direction {
        StepDirection::Advance => retreat_destination(destination, player),
        StepDirection::Retreat => advance_destination(destination, player),
    }
}

const fn legal_retreater_drop(player: Player, destination: Rank) -> bool {
    match player {
        Player::Alpha => matches!(destination, Rank::R1 | Rank::R2 | Rank::R3 | Rank::R4),
        Player::Beta => matches!(destination, Rank::R3 | Rank::R4 | Rank::R5 | Rank::R6),
    }
}

const fn next_rank(rank: Rank) -> Option<Rank> {
    match rank {
        Rank::R1 => Some(Rank::R2),
        Rank::R2 => Some(Rank::R3),
        Rank::R3 => Some(Rank::R4),
        Rank::R4 => Some(Rank::R5),
        Rank::R5 => Some(Rank::R6),
        Rank::R6 => None,
    }
}

const fn previous_rank(rank: Rank) -> Option<Rank> {
    match rank {
        Rank::R1 => None,
        Rank::R2 => Some(Rank::R1),
        Rank::R3 => Some(Rank::R2),
        Rank::R4 => Some(Rank::R3),
        Rank::R5 => Some(Rank::R4),
        Rank::R6 => Some(Rank::R5),
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
struct CacheKey {
    position: Position,
    ply_from_root: u16,
}

impl Hash for CacheKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.position.hash(state);
        state.write_u16(self.ply_from_root);
    }
}

struct FastHasher {
    value: u64,
}

impl Default for FastHasher {
    fn default() -> Self {
        Self {
            value: 0x517C_C1B7_2722_0A95,
        }
    }
}

impl FastHasher {
    fn mix(&mut self, value: u64) {
        self.value ^= value.wrapping_mul(0x9E37_79B9_7F4A_7C15);
        self.value = self
            .value
            .rotate_left(27)
            .wrapping_mul(0x94D0_49BB_1331_11EB);
    }
}

impl Hasher for FastHasher {
    fn finish(&self) -> u64 {
        self.value ^ (self.value >> 29)
    }

    fn write(&mut self, bytes: &[u8]) {
        let mut chunks = bytes.chunks_exact(8);
        for chunk in &mut chunks {
            self.mix(u64::from_le_bytes(chunk.try_into().unwrap()));
        }
        let remainder = chunks.remainder();
        if !remainder.is_empty() {
            let mut tail = [0; 8];
            tail[..remainder.len()].copy_from_slice(remainder);
            self.mix(u64::from_le_bytes(tail) ^ remainder.len() as u64);
        }
    }

    fn write_u16(&mut self, value: u16) {
        self.mix(u64::from(value));
    }

    fn write_u64(&mut self, value: u64) {
        self.mix(value);
    }
}

type TranspositionTable = HashMap<CacheKey, TableEntry, BuildHasherDefault<FastHasher>>;

#[derive(Clone, Copy)]
enum Bound {
    Exact,
    Lower,
    Upper,
}

#[derive(Clone, Copy)]
struct TableEntry {
    depth: u16,
    score: i32,
    bound: Bound,
    best: Option<Ply>,
}

struct ScoredPly {
    ply: Ply,
    order: i32,
}

#[derive(Clone, Copy)]
struct NodeResult {
    score: i32,
    best: Option<Ply>,
}

enum SearchStatus {
    Complete(NodeResult),
    Incomplete,
}

struct NodeBudget {
    remaining: u32,
}

impl NodeBudget {
    fn new(nodes: u32) -> Self {
        Self { remaining: nodes }
    }

    fn take(&mut self) -> bool {
        if self.remaining == 0 {
            return false;
        }
        self.remaining -= 1;
        true
    }
}

struct SearchCore {
    table: TranspositionTable,
    move_buffers: Vec<Vec<ScoredPly>>,
    pv_hint: Vec<Ply>,
}

impl SearchCore {
    fn new() -> Self {
        Self {
            table: HashMap::with_capacity_and_hasher(16_384, BuildHasherDefault::default()),
            move_buffers: Vec::with_capacity(16),
            pv_hint: Vec::with_capacity(16),
        }
    }

    fn reset(&mut self) {
        self.table.clear();
        self.pv_hint.clear();
        for moves in &mut self.move_buffers {
            moves.clear();
        }
    }

    fn begin_iteration(&mut self, previous_pv: &[Ply]) {
        self.table.clear();
        self.pv_hint.clear();
        self.pv_hint.extend_from_slice(previous_pv);
    }

    fn prepare_moves(&mut self, state: Position, level: usize, preferred: Option<Ply>) -> usize {
        while self.move_buffers.len() <= level {
            self.move_buffers
                .push(Vec::with_capacity(INITIAL_MOVE_CAPACITY));
        }
        let moves = &mut self.move_buffers[level];
        generate_plies(state, preferred, moves);
        moves.sort_unstable_by_key(|candidate| Reverse(candidate.order));
        moves.len()
    }

    fn store(&mut self, key: CacheKey, entry: TableEntry) {
        match self.table.entry(key) {
            Entry::Occupied(mut occupied) => {
                let old = occupied.get();
                if entry.depth > old.depth
                    || (entry.depth == old.depth
                        && matches!(entry.bound, Bound::Exact)
                        && !matches!(old.bound, Bound::Exact))
                {
                    occupied.insert(entry);
                }
            }
            Entry::Vacant(vacant) => {
                vacant.insert(entry);
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn search(
        &mut self,
        state: Position,
        depth: u16,
        ply_from_root: u16,
        level: usize,
        mut alpha: i32,
        mut beta: i32,
        on_previous_pv: bool,
        budget: &mut NodeBudget,
    ) -> SearchStatus {
        let key = CacheKey {
            position: state,
            ply_from_root,
        };
        let original_alpha = alpha;
        let original_beta = beta;
        let cached = self.table.get(&key).copied();
        let mut preferred = cached.and_then(|entry| entry.best);

        if let Some(entry) = cached.filter(|entry| entry.depth >= depth) {
            match entry.bound {
                Bound::Exact => {
                    return SearchStatus::Complete(NodeResult {
                        score: entry.score,
                        best: entry.best,
                    });
                }
                Bound::Lower => alpha = alpha.max(entry.score),
                Bound::Upper => beta = beta.min(entry.score),
            }
            if alpha >= beta {
                return SearchStatus::Complete(NodeResult {
                    score: entry.score,
                    best: entry.best,
                });
            }
        }

        if !budget.take() {
            return SearchStatus::Incomplete;
        }

        if let Some(winner) = state.captured_snipe_winner() {
            let score = terminal_score(winner, ply_from_root);
            self.store(
                key,
                TableEntry {
                    depth,
                    score,
                    bound: Bound::Exact,
                    best: None,
                },
            );
            return SearchStatus::Complete(NodeResult { score, best: None });
        }

        if depth == 0 {
            let mut actions = ActionBuffer::new();
            state.write_legal_actions(&mut actions);
            let score = if actions.is_empty() {
                terminal_score(state.active_player.opponent(), ply_from_root)
            } else {
                evaluate(state, actions.len)
            };
            self.store(
                key,
                TableEntry {
                    depth,
                    score,
                    bound: Bound::Exact,
                    best: None,
                },
            );
            return SearchStatus::Complete(NodeResult { score, best: None });
        }

        if preferred.is_none() && on_previous_pv {
            preferred = self.pv_hint.get(level).copied();
        }
        let move_count = self.prepare_moves(state, level, preferred);
        if move_count == 0 {
            let score = terminal_score(state.active_player.opponent(), ply_from_root);
            self.store(
                key,
                TableEntry {
                    depth,
                    score,
                    bound: Bound::Exact,
                    best: None,
                },
            );
            return SearchStatus::Complete(NodeResult { score, best: None });
        }

        let maximizing = state.active_player == Player::Alpha;
        let mut best_score = if maximizing { -INFINITY } else { INFINITY };
        let mut best_ply = None;

        for index in 0..move_count {
            let ply = self.move_buffers[level][index].ply;
            let child = apply_ply(state, ply).expect("generated ply is legal");
            let child_on_previous_pv =
                on_previous_pv && self.pv_hint.get(level).is_some_and(|hint| *hint == ply);
            let child_result = match self.search(
                child,
                depth - 1,
                ply_from_root.saturating_add(1),
                level + 1,
                alpha,
                beta,
                child_on_previous_pv,
                budget,
            ) {
                SearchStatus::Complete(result) => result,
                SearchStatus::Incomplete => return SearchStatus::Incomplete,
            };

            if best_ply.is_none()
                || (maximizing && child_result.score > best_score)
                || (!maximizing && child_result.score < best_score)
            {
                best_score = child_result.score;
                best_ply = Some(ply);
            }
            if maximizing {
                alpha = alpha.max(best_score);
            } else {
                beta = beta.min(best_score);
            }
            if alpha >= beta {
                break;
            }
        }

        let bound = if best_score <= original_alpha {
            Bound::Upper
        } else if best_score >= original_beta {
            Bound::Lower
        } else {
            Bound::Exact
        };
        self.store(
            key,
            TableEntry {
                depth,
                score: best_score,
                bound,
                best: best_ply,
            },
        );
        SearchStatus::Complete(NodeResult {
            score: best_score,
            best: best_ply,
        })
    }

    fn write_principal_variation(&self, root: Position, depth: u16, result: &mut Vec<Ply>) {
        result.clear();
        result.reserve(usize::from(depth));
        let mut state = root;
        for ply_from_root in 0..depth {
            let key = CacheKey {
                position: state,
                ply_from_root,
            };
            let Some(ply) = self.table.get(&key).and_then(|entry| entry.best) else {
                break;
            };
            let Some(child) = apply_ply(state, ply) else {
                break;
            };
            result.push(ply);
            state = child;
            if state.winner().is_some() {
                break;
            }
        }
    }

    fn fallback(&mut self, state: Position) -> Option<Ply> {
        self.prepare_moves(state, 0, None);
        self.move_buffers[0].first().map(|candidate| candidate.ply)
    }
}

/// A deterministic analyzer built around resumable, sound alpha-beta search.
pub struct AvocadoAnalyzer {
    root: Option<Position>,
    search: SearchCore,
    target_depth: u16,
    completed_depth: u16,
    published_pv: Vec<Ply>,
    evaluation: Evaluation,
    terminal: bool,
}

impl Default for AvocadoAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl AvocadoAnalyzer {
    pub fn new() -> Self {
        Self {
            root: None,
            search: SearchCore::new(),
            target_depth: 1,
            completed_depth: 0,
            published_pv: Vec::with_capacity(16),
            evaluation: estimate(0),
            terminal: false,
        }
    }

    /// Deepest fully completed whole-ply search.
    pub const fn completed_depth(&self) -> u16 {
        self.completed_depth
    }
}

impl Analyzer for AvocadoAnalyzer {
    fn set_state(&mut self, state: State) {
        self.search.reset();
        self.target_depth = 1;
        self.completed_depth = 0;
        self.published_pv.clear();
        self.terminal = false;

        let position = Position::from_core(&state);
        if let Some(winner) = position.winner() {
            self.evaluation = mate_in(winner, 0);
            self.terminal = true;
        } else {
            let mut actions = ActionBuffer::new();
            position.write_legal_actions(&mut actions);
            self.evaluation = estimate(evaluate(position, actions.len));
            if let Some(fallback) = self.search.fallback(position) {
                self.published_pv.push(fallback);
            }
        }
        self.root = Some(position);
    }

    fn think_for_one_tick(&mut self) {
        if self.terminal {
            return;
        }
        let Some(root) = self.root else {
            return;
        };
        let mut budget = NodeBudget::new(NODES_PER_TICK);
        let result = self.search.search(
            root,
            self.target_depth,
            0,
            0,
            -INFINITY,
            INFINITY,
            true,
            &mut budget,
        );
        let SearchStatus::Complete(result) = result else {
            return;
        };

        self.search
            .write_principal_variation(root, self.target_depth, &mut self.published_pv);
        if self.published_pv.is_empty()
            && let Some(best) = result.best
        {
            self.published_pv.push(best);
        }
        self.evaluation = score_to_evaluation(result.score);
        self.completed_depth = self.target_depth;
        self.target_depth = self.target_depth.saturating_add(1);
        self.search.begin_iteration(&self.published_pv);
    }

    fn evaluation(&self) -> Evaluation {
        self.evaluation
    }

    fn write_optimal_lop<W: ActionWriter>(&self, writer: &mut W) {
        writer.reserve(self.published_pv.len().saturating_mul(2));
        for &ply in &self.published_pv {
            ply.write(writer);
        }
    }
}

fn generate_plies(state: Position, preferred: Option<Ply>, output: &mut Vec<ScoredPly>) {
    output.clear();
    let mover = state.active_player;
    let mut first_actions = ActionBuffer::new();
    let mut second_actions = ActionBuffer::new();
    state.write_legal_actions(&mut first_actions);

    for &first in first_actions.as_slice() {
        let first_order = action_order(state, first);
        if first.is_standalone_ply() {
            push_scored_ply(
                Ply {
                    first,
                    second: None,
                },
                preferred,
                first_order,
                output,
            );
            continue;
        }
        let Some(after_first) = state.apply(first) else {
            continue;
        };
        if after_first.active_player != mover || after_first.captured_snipe_winner().is_some() {
            push_scored_ply(
                Ply {
                    first,
                    second: None,
                },
                preferred,
                first_order,
                output,
            );
            continue;
        }

        second_actions.clear();
        after_first.write_legal_actions(&mut second_actions);
        if second_actions.is_empty() {
            push_scored_ply(
                Ply {
                    first,
                    second: None,
                },
                preferred,
                first_order,
                output,
            );
            continue;
        }
        for &second in second_actions.as_slice() {
            push_scored_ply(
                Ply {
                    first,
                    second: Some(second),
                },
                preferred,
                first_order + action_order(after_first, second),
                output,
            );
        }
    }
}

fn push_scored_ply(ply: Ply, preferred: Option<Ply>, base_order: i32, output: &mut Vec<ScoredPly>) {
    let mut order = base_order;
    if preferred == Some(ply) {
        order += 1_000_000;
    }
    if ply.second.is_some() {
        order += 100;
    }
    output.push(ScoredPly { ply, order });
}

fn action_order(state: Position, action: Action) -> i32 {
    match action {
        Action::AnimalStep(step) => {
            let destination = state.rank(step.destination);
            if would_activate(step.actor, destination) {
                let captured = i32::try_from(destination.card_total()).unwrap();
                let wins = destination.has_snipe(state.active_player.opponent());
                50_000 + captured * 2_000 + if wins { 700_000 } else { 0 }
            } else {
                1_000
            }
        }
        Action::Drop(_) => 100,
        Action::SnipeStep(_) => 0,
    }
}

fn apply_ply(state: Position, ply: Ply) -> Option<Position> {
    let mut child = state.apply(ply.first)?;
    if let Some(second) = ply.second {
        child = child.apply(second)?;
    }
    Some(child)
}

fn terminal_score(winner: Player, ply_from_root: u16) -> i32 {
    let distance = i32::from(ply_from_root);
    if winner == Player::Alpha {
        MATE_SCORE - distance
    } else {
        -MATE_SCORE + distance
    }
}

fn evaluate(state: Position, mobility: usize) -> i32 {
    let mut score = 0i32;

    score += i32::try_from(state.reserve().animal_total(Player::Alpha)).unwrap() * 2_200;
    score -= i32::try_from(state.reserve().animal_total(Player::Beta)).unwrap() * 2_200;

    for (rank_index, rank) in RANKS.into_iter().enumerate() {
        let cards = state.rank(rank);
        let alpha_progress = rank_index as i32 * 90;
        let beta_progress = (5 - rank_index) as i32 * 90;
        score +=
            i32::try_from(cards.animal_total(Player::Alpha)).unwrap() * (2_000 + alpha_progress);
        score -= i32::try_from(cards.animal_total(Player::Beta)).unwrap() * (2_000 + beta_progress);

        let support = (i32::try_from(cards.card_total()).unwrap() - 1).max(0) * 140;
        if cards.has_snipe(Player::Alpha) {
            score += (5 - rank_index) as i32 * 260 + support;
        }
        if cards.has_snipe(Player::Beta) {
            score -= rank_index as i32 * 260 + support;
        }
    }

    let mobility = i32::try_from(mobility).unwrap_or(i32::MAX);
    if state.active_player == Player::Alpha {
        score += mobility * 12 + 40;
    } else {
        score -= mobility * 12 + 40;
    }
    score.clamp(
        EvaluationEstimate::MIN.millipoints(),
        EvaluationEstimate::MAX.millipoints(),
    )
}

fn score_to_evaluation(score: i32) -> Evaluation {
    if score.abs() >= MATE_FLOOR {
        let winner = if score > 0 {
            Player::Alpha
        } else {
            Player::Beta
        };
        let plies = u32::try_from(MATE_SCORE - score.abs()).unwrap_or(0);
        mate_in(winner, plies)
    } else {
        estimate(score)
    }
}

fn mate_in(winner: Player, plies: u32) -> Evaluation {
    MateInN::new(winner, plies)
        .expect("Avocado's mate distance is supported by snipe-core")
        .into()
}

fn estimate(millipoints: i32) -> Evaluation {
    EvaluationEstimate::from_millipoints(millipoints)
        .expect("Avocado clamps heuristic evaluations to the public range")
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use snipe_core::{AnimalDrop, AnimalStep, CardMultiset, SnipeStep, StepDirection};
    use snipe_prng::initial_state;

    fn cards(cards: &[(Card, Player)]) -> CardMultiset {
        cards
            .iter()
            .fold(CardMultiset::EMPTY, |result, &(card, player)| {
                result
                    .checked_add(CardMultiset::singleton(card, player))
                    .unwrap()
            })
    }

    fn animal(name: &str) -> Animal {
        match name {
            "Rat" => Animal::Mouse,
            "Ox" => Animal::Ox,
            "Tiger" => Animal::Tiger,
            "Rabbit" => Animal::Rabbit,
            "Dragon" => Animal::Dragon,
            "Snake" => Animal::Snake,
            "Horse" => Animal::Horse,
            "Ram" => Animal::Ram,
            "Monkey" => Animal::Monkey,
            "Rooster" => Animal::Rooster,
            "Dog" => Animal::Dog,
            "Boar" => Animal::Boar,
            "Fish" => Animal::Fish,
            "Elephant" => Animal::Elephant,
            "Squid" => Animal::Squid,
            "Frog" => Animal::Frog,
            _ => panic!("unknown animal {name}"),
        }
    }

    fn rank(notation: &str) -> Rank {
        match notation
            .bytes()
            .find(u8::is_ascii_digit)
            .expect("move notation has a rank")
        {
            b'1' => Rank::R1,
            b'2' => Rank::R2,
            b'3' => Rank::R3,
            b'4' => Rank::R4,
            b'5' => Rank::R5,
            b'6' => Rank::R6,
            value => panic!("invalid rank {}", char::from(value)),
        }
    }

    fn action(notation: &str) -> Action {
        let (actor, movement) = notation
            .split_once(' ')
            .expect("move notation has an actor and movement");
        let destination = rank(movement);
        if actor == "Alpha" || actor == "Beta" {
            Action::SnipeStep(SnipeStep { destination })
        } else if movement.starts_with('&') {
            Action::Drop(AnimalDrop {
                actor: animal(actor),
                destination,
            })
        } else {
            Action::AnimalStep(AnimalStep {
                actor: animal(actor),
                direction: if movement.starts_with('*') {
                    StepDirection::Retreat
                } else {
                    StepDirection::Advance
                },
                destination,
            })
        }
    }

    fn bug_position() -> State {
        const HISTORY: &str = "\
1b. Ox 4, Fish 4x
2a. Dragon 3, Squid *1
3b. Dog 4, Boar 4
4a. Ram 3, Boar 3
5b. Tiger 4x, Horse 5
6a. Ox 3, Tiger 3
7b. Elephant &6
8a. Rabbit *1, Squid 2
9b. Ox &4
10a. Ram *1, Boar *2
11b. Ox 3, Rooster 4
12a. Fish 3, Squid 3x
13b. Tiger 3, Rat 4
14a. Snake 3, Squid 4
15b. Dog &2
16a. Tiger &2
17b. Tiger 2x, Rat 3
18a. Rooster 2, Squid 5
19b. Boar &5
20a. Rabbit 2, Rooster 3
21b. Boar &3
22a. Dragon &2
23b. Rat 2, Boar 2x
24a. Fish &6
25b. Rat 4, Elephant 5x
26a. Dragon &2
27b. Rooster 3, Boar 1
28a. Monkey 2, Snake 4
29b. Dragon &1
30a. Rooster 4, Snake 5
31b. Horse &2";
        let mut state = initial_state(0);
        for line in HISTORY.lines() {
            let (_, moves) = line.split_once(". ").expect("history line has a prefix");
            for notation in moves.split(", ") {
                state = state
                    .apply(action(notation))
                    .unwrap_or_else(|error| panic!("{line}: {notation} is illegal: {error:?}"));
            }
        }
        state
    }

    fn test_plies(state: &State) -> Vec<(Ply, State)> {
        let player = state.active_player;
        let mut result = Vec::new();
        let mut first_actions = Vec::new();
        state.write_legal_actions(&mut first_actions);
        for first in first_actions {
            let after_first = state.clone().apply(first).unwrap();
            if after_first.active_player != player || after_first.winner().is_some() {
                result.push((
                    Ply {
                        first,
                        second: None,
                    },
                    after_first,
                ));
                continue;
            }
            let mut second_actions = Vec::new();
            after_first.write_legal_actions(&mut second_actions);
            if second_actions.is_empty() {
                result.push((
                    Ply {
                        first,
                        second: None,
                    },
                    after_first,
                ));
                continue;
            }
            for second in second_actions {
                let child = after_first.clone().apply(second).unwrap();
                result.push((
                    Ply {
                        first,
                        second: Some(second),
                    },
                    child,
                ));
            }
        }
        result
    }

    fn is_forced_mate(state: &State, winner: Player, plies: u32) -> bool {
        if let Some(actual) = state.winner() {
            return actual == winner && plies == 0;
        }
        if plies == 0 {
            return false;
        }
        let moves = test_plies(state);
        if state.active_player == winner {
            moves
                .iter()
                .any(|(_, child)| is_forced_mate(child, winner, plies - 1))
        } else {
            moves
                .iter()
                .all(|(_, child)| is_forced_mate(child, winner, plies - 1))
        }
    }

    #[test]
    fn packed_rules_match_the_reference_across_random_play() {
        for seed in 0..32 {
            let mut reference = initial_state(seed);
            let mut packed = Position::from_core(&reference);
            let mut random = seed ^ 0xA70C_AD00_5EED;

            for _ in 0..160 {
                assert_eq!(packed, Position::from_core(&reference));
                assert_eq!(packed.winner(), reference.winner());

                let mut reference_actions = Vec::new();
                let mut packed_actions = ActionBuffer::new();
                reference.write_legal_actions(&mut reference_actions);
                packed.write_legal_actions(&mut packed_actions);
                assert_eq!(
                    packed_actions.as_slice(),
                    reference_actions,
                    "legal actions diverged for seed {seed}"
                );
                if reference_actions.is_empty() {
                    break;
                }

                random = snipe_prng::splitmix64(random);
                let action = reference_actions[random as usize % reference_actions.len()];
                reference = reference.apply(action).unwrap();
                packed = packed.apply(action).expect("reference-legal action");
            }
        }
    }

    #[test]
    fn reports_an_immediate_forced_mate_truthfully() {
        let state = State {
            active_player: Player::Alpha,
            reserves: CardMultiset::EMPTY,
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
            r6: CardMultiset::EMPTY,
            leading_action: None,
        };
        let mut analyzer = AvocadoAnalyzer::new();
        analyzer.set_state(state.clone());
        analyzer.think_for_one_tick();

        let Evaluation::MateInN(mate) = analyzer.evaluation() else {
            panic!("expected a forced mate");
        };
        assert_eq!(mate.winner(), Player::Alpha);
        assert_eq!(mate.plies(), 1);
        assert!(is_forced_mate(&state, mate.winner(), mate.plies()));
    }

    #[test]
    fn never_publishes_an_interrupted_search() {
        let state = bug_position();
        let mut search = SearchCore::new();
        let mut budget = NodeBudget::new(1);
        assert!(matches!(
            search.search(
                Position::from_core(&state),
                3,
                0,
                0,
                -INFINITY,
                INFINITY,
                true,
                &mut budget
            ),
            SearchStatus::Incomplete
        ));
    }

    #[test]
    fn ram_four_is_not_mate_in_three() {
        let root = bug_position();
        let after_ram = root.apply(action("Ram &4")).unwrap();
        let replies = test_plies(&after_ram);
        let escaping_replies = replies
            .iter()
            .filter(|(_, child)| {
                !test_plies(child)
                    .iter()
                    .any(|(_, grandchild)| grandchild.winner() == Some(Player::Alpha))
            })
            .count();

        assert_eq!(replies.len(), 84);
        assert_eq!(escaping_replies, 80);
    }

    #[test]
    fn bug_position_does_not_regress_to_a_false_mate() {
        let state = bug_position();
        let mut analyzer = AvocadoAnalyzer::new();
        analyzer.set_state(state.clone());
        for _ in 0..2_000 {
            analyzer.think_for_one_tick();
            if analyzer.completed_depth >= 3 {
                break;
            }
        }
        assert!(
            analyzer.completed_depth >= 3,
            "depth-three search did not finish"
        );
        if let Evaluation::MateInN(mate) = analyzer.evaluation() {
            assert!(
                is_forced_mate(&state, mate.winner(), mate.plies()),
                "Avocado published an unforced {mate:?}"
            );
        }
    }

    #[test]
    fn writes_only_legal_actions_from_the_completed_iteration() {
        let state = initial_state(7);
        let player = state.active_player;
        let mut analyzer = AvocadoAnalyzer::new();
        analyzer.set_state(state.clone());
        analyzer.think(8);

        let mut actions = Vec::new();
        analyzer.write_optimal_lop(&mut actions);
        assert!(!actions.is_empty());
        let mut replay = state;
        let mut completed = false;
        for action in actions {
            replay = replay.apply(action).unwrap();
            if replay.active_player != player || replay.winner().is_some() {
                completed = true;
            }
        }
        assert!(completed);
    }

    #[test]
    fn terminal_positions_are_mate_in_zero_without_thinking() {
        let state = State {
            active_player: Player::Beta,
            reserves: cards(&[(Card::Snipe, Player::Beta)]),
            r1: cards(&[(Card::Snipe, Player::Alpha)]),
            r2: CardMultiset::EMPTY,
            r3: CardMultiset::EMPTY,
            r4: CardMultiset::EMPTY,
            r5: CardMultiset::EMPTY,
            r6: CardMultiset::EMPTY,
            leading_action: None,
        };
        let mut analyzer = AvocadoAnalyzer::new();
        analyzer.set_state(state);
        assert_eq!(
            analyzer.evaluation(),
            MateInN::new(Player::Alpha, 0).unwrap().into()
        );
        let mut line = Vec::new();
        analyzer.write_optimal_lop(&mut line);
        assert!(line.is_empty());
    }
}
