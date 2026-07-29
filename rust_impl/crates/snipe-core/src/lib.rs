//! This crate is meant to be an authoritative reference implementation.
//! It is NOT time-efficient or space-efficient.
//! Instead, it prioritizes a clean interface and developer ergonomics.
//!
//! Analyzers should only use this crate at public interface boundaries,
//! while internally using more efficient algorithms/data-structures.

// Note: This file is just the "header" file, outlining the public interface.
// All the non-trivial implementations are in the `private_impl` module.

pub use std::{
    cmp::Ordering,
    fmt::{self, Debug},
};

mod private_impl;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Player {
    Alpha,
    Beta,
}
impl Player {
    pub const fn opponent(self) -> Self {
        match self {
            Self::Alpha => Self::Beta,
            Self::Beta => Self::Alpha,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Rank {
    R1,
    R2,
    R3,
    R4,
    R5,
    R6,
}

/// This deliberately uses a sub-optimal memory representation,
/// in order to improve developer ergonomics.
/// Consequently, it is not `Copy`.
///
/// Also, it allows a `leading_action`, which further improves developer ergonomics
/// at the cost of performance.
///
/// If you are implementing your own `Agent`, we recommend you create your own, more efficient representation.
#[derive(Clone)]
pub struct State {
    pub active_player: Player,
    pub reserves: CardMultiset,
    pub r1: CardMultiset,
    pub r2: CardMultiset,
    pub r3: CardMultiset,
    pub r4: CardMultiset,
    pub r5: CardMultiset,
    pub r6: CardMultiset,
    pub leading_action: Option<AnimalStep>,
}
impl Debug for State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.fmt_(f)
    }
}
impl State {
    pub fn write_legal_actions<W>(&self, w: &mut W)
    where
        W: ActionWriter,
    {
        self.write_legal_actions_(w);
    }

    pub fn apply(self, action: Action) -> Result<Self, IllegalActionError> {
        self.apply_(action)
    }

    pub fn winner(&self) -> Option<Player> {
        self.winner_()
    }
}

#[derive(Debug, Clone)]
pub struct InitialStateBuilder {
    pub alpha_reserve: [Animal; 1],
    pub r1: [Animal; 2],
    pub r2: [Animal; 12],
    pub r3: [Animal; 1],
    pub r4: [Animal; 1],
    pub r5: [Animal; 12],
    pub r6: [Animal; 2],
    pub beta_reserve: [Animal; 1],
}
impl InitialStateBuilder {
    /// This validates the animal counts.
    /// There must be exactly 2 of each animal, or else this returns `None`.
    pub const fn build(self) -> Option<State> {
        self.build_()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IllegalActionError {
    SnipeAlreadyCaptured,
    AlreadyMovedAnimal,
    StepDestinationOutOfRange,
    CannotEmptyRowWithoutImmediatelyWinning,
    DroppedAnimalNotInReserve,
    CannotEmptyReserve,
    CannotDropRetreaterOnEnemyBackTwoRanks,
    MovedCardInReserve,
    CardNotFound,
    NotYourAnimal,
    CannotMoveSameAnimalTwice,
    CannotCaptureOwnSnipeWithoutAlsoCapturingOpponent,
}

pub trait ActionWriter {
    fn push(&mut self, action: Action);

    fn reserve(&mut self, additional: usize);
}

// There are 2 of each animal in the deck.
// Therefore, for any animal, there can either be:
// - 0 of them (bitpattern: `000`)
// - 1 Alpha (bitpattern: `100`)
// - 1 Beta (`010`)
// - 1 of each (2 total) (bitpattern: `110`)
// - 2 Alpha (bitpattern: `101`)
// - 2 Beta (bitpattern: `011`)
// Any other bitpattern form is illegal.
// The bitpatterns above are written `{alpha_presence}{beta_presence}{has_allied_twins}`.
#[derive(Clone, Copy)]
pub struct CardMultiset {
    /// A bit vector where the i-th bit answers "Is there at least one {i-th animal} allegiant to Alpha?"
    alpha_presence: u16,

    /// A bit vector where the i-th bit answers "Is there at least one {i-th animal} allegiant to Beta?"
    beta_presence: u16,

    /// A bit vector where the i-th bit answers "Does this contain 2 {i-th animal}s, and both are allegiant to the same player?"
    has_allied_twins: u16,

    /// Bit0 is Alpha, Bit1 is Beta, the others are "don't care" bits.
    snipes: u8,
}
impl Debug for CardMultiset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.fmt_(f)
    }
}
impl CardMultiset {
    pub const EMPTY: Self = Self {
        alpha_presence: 0,
        beta_presence: 0,
        has_allied_twins: 0,
        snipes: 0,
    };

    pub const fn singleton(card: Card, allegiance: Player) -> Self {
        Self::singleton_(card, allegiance)
    }

    pub const fn count(self, card: Card, allegiance: Player) -> u8 {
        self.count_(card, allegiance)
    }

    /// Returns `None` if the sum would have an illegal number of any of the card types.
    /// There can be at most 2 of any animal (regardless of allegiance), at most 1 Alpha Snipe, and at most 1 Beta Snipe.
    pub const fn checked_add(self, other: Self) -> Option<Self> {
        self.checked_add_(other)
    }

    /// Returns `None` if the card (of that allegiance) is not present
    pub const fn remove_one(self, card: Card, allegiance: Player) -> Option<Self> {
        self.remove_one_(card, allegiance)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AnimalStep {
    pub actor: Animal,
    pub direction: StepDirection,
    pub destination: Rank,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StepDirection {
    Advance,
    Retreat,
}

#[derive(Debug, Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Animal {
    Mouse,
    Ox,
    Tiger,
    Rabbit,
    Dragon,
    Snake,
    Horse,
    Ram,
    Monkey,
    Rooster,
    Dog,
    Boar,
    Fish,
    Elephant,
    Squid,
    Frog,
}
impl Animal {
    pub const fn is_retreater(self) -> bool {
        self.is_retreater_()
    }

    pub const fn unary_element(self) -> Option<Element> {
        self.unary_element_()
    }

    pub const fn binary_element(self) -> Option<Element> {
        self.binary_element_()
    }

    pub const fn ternary_element(self) -> Option<Element> {
        self.ternary_element_()
    }

    pub const fn would_activate_triplet_by_entering(self, destination: CardMultiset) -> bool {
        self.would_activate_triplet_by_entering_(destination)
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum Element {
    Fire,
    Water,
    Earth,
    Air,
}

#[derive(Debug, Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Card {
    Animal(Animal),
    Snipe,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    AnimalStep(AnimalStep),
    SnipeStep(SnipeStep),
    Drop(AnimalDrop),
}
impl Action {
    pub const fn is_standalone_ply(self) -> bool {
        matches!(self, Self::SnipeStep(_) | Self::Drop(_))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SnipeStep {
    pub destination: Rank,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AnimalDrop {
    pub actor: Animal,
    pub destination: Rank,
}

/// An on-line position analyzer.
/// The consumer can initialize with `set_state`,
/// and then think for as many ticks as it wants.
/// For example, `while elapsed_time() < LIMIT { analyzer.think(100); }`.
/// (Usually, the consumer tracks either tick count, physical clock time, or both.)
///
/// "On-line" means that you can receive an analysis after any number of _thinking ticks_ (including zero).
/// The quality of the analysis will generally improve as you think for more ticks.
/// A tick is an atomic unit of work.
pub trait Analyzer {
    fn set_state(&mut self, state: State);

    fn think(&mut self, ticks: usize) {
        for _ in 0..ticks {
            self.think_for_one_tick();
        }
    }

    /// - Since ticks are blocking, they should complete fairly quickly.
    /// - All ticks should take "roughly" (usually we mean "asymptotically") the same amount of time.
    ///
    /// The second requirement has a lot of wiggle room, so you rarely have to worry about it in practice.
    /// Just make sure to not do anything unreasonable (e.g., define 1tick=1depth of IDFS, which would make each tick take exponentially longer than the previous).
    /// In practice, satisfying the first requirement will almost always automatically satisfy the second requirement.
    fn think_for_one_tick(&mut self);

    /// Your implementation must be truthful about mates.
    /// In other words, an implementation is only sound if it satisfies ALL of the following:
    /// - If the game is over, you must return mate-in-zero.
    /// - If you return mate-in-N, there must truly exist a forced win in N plies.
    ///   - As a corollary, if you return mate-in-zero, the game must truly be over.
    /// - If you return an estimate, the game must NOT be over.
    fn evaluation(&self) -> Evaluation;

    /// "LOP" stands for "line of play".
    /// This must only write legal actions.
    /// Usually, the more actions this writes, the more helpful it is.
    /// At a _minimum_, if the game is not over, this must write enough actions to either complete the active player's ply,
    /// or end the game.
    ///
    /// If `evaluation` returns a mate-in-N, it is **strongly** recommended (but technically not required)
    /// that `write_optimal_lop` write the entire winning line of play.
    fn write_optimal_lop<W>(&self, w: &mut W)
    where
        W: ActionWriter;
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum Evaluation {
    MateInN(MateInN),
    Estimate(EvaluationEstimate),
}
impl Ord for Evaluation {
    fn cmp(&self, other: &Self) -> Ordering {
        self.cmp_(other)
    }
}
impl PartialOrd for Evaluation {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Debug for Evaluation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.fmt_(f)
    }
}
impl Evaluation {
    pub const fn compress(self) -> CompressedEvaluation {
        self.compress_()
    }
}
impl From<MateInN> for Evaluation {
    fn from(value: MateInN) -> Self {
        Self::MateInN(value)
    }
}
impl From<EvaluationEstimate> for Evaluation {
    fn from(value: EvaluationEstimate) -> Self {
        Self::Estimate(value)
    }
}
impl From<CompressedEvaluation> for Evaluation {
    fn from(value: CompressedEvaluation) -> Self {
        value.decompress()
    }
}

/// "Mate-in-zero" means the game is already over.
/// This can happen because a snipe was captured or because the active player has no legal actions.
#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub struct MateInN {
    winner: Player,
    plies: u32,
}
impl Ord for MateInN {
    fn cmp(&self, other: &Self) -> Ordering {
        self.cmp_(other)
    }
}
impl PartialOrd for MateInN {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Debug for MateInN {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.fmt_(f)
    }
}
impl MateInN {
    pub const MAX_PLIES: u32 = 1_000_000;

    pub const fn new(winner: Player, plies: u32) -> Option<Self> {
        if plies <= Self::MAX_PLIES {
            return Some(Self { winner, plies });
        }

        None
    }

    pub const fn winner(self) -> Player {
        self.winner
    }

    pub const fn plies(self) -> u32 {
        self.plies
    }

    pub const fn compress(self) -> CompressedEvaluation {
        self.compress_()
    }
}

/// `+100.000` is the most favorable score for Alpha;
/// `-100.000` is the most favorable score for Beta.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EvaluationEstimate {
    /// INVARIANT: This must be within `-100_000..=100_000`.
    millipoints: i32,
}
impl EvaluationEstimate {
    pub const MIN: Self = Self {
        millipoints: -100_000,
    };

    pub const MAX: Self = Self {
        millipoints: 100_000,
    };

    pub const ZERO: Self = Self { millipoints: 0 };

    pub const fn from_millipoints(millipoints: i32) -> Option<Self> {
        if Self::MIN.millipoints <= millipoints && millipoints <= Self::MAX.millipoints {
            return Some(Self { millipoints });
        }

        None
    }

    pub const fn millipoints(self) -> i32 {
        self.millipoints
    }

    pub const fn compress(self) -> CompressedEvaluation {
        self.compress_()
    }
}
impl Debug for EvaluationEstimate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.fmt_(f)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CompressedEvaluation {
    /// - If this is within `-100_000..=100_000`, it represents an EvaluationEstimate.
    /// - If this is within `1_000_000..=2_000_000`, it represents mate-in-`2_000_000 - raw`, with Alpha winning.
    /// - If this is within `-2_000_000..=-1_000_000`, it represents mate-in-`2_000_000 + raw`, with Beta winning.
    raw: i32,
}
impl From<Evaluation> for CompressedEvaluation {
    fn from(value: Evaluation) -> Self {
        value.compress()
    }
}
impl From<MateInN> for CompressedEvaluation {
    fn from(value: MateInN) -> Self {
        value.compress()
    }
}
impl From<EvaluationEstimate> for CompressedEvaluation {
    fn from(value: EvaluationEstimate) -> Self {
        value.compress()
    }
}
impl Debug for CompressedEvaluation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.fmt_(f)
    }
}
impl CompressedEvaluation {
    pub const fn decompress(self) -> Evaluation {
        self.decompress_()
    }
}
