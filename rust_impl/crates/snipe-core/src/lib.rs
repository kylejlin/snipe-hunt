//! This crate meant to be an authoritative reference implementation.
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
/// Also, it allows an `leading_action`, which further improves developer ergonomics
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

/// A (roughly) on-line position analyzer.
/// The consumer can initialize with `set_state`,
/// and then `think` for as long as it wants
/// (usually, `on_tick_complete` tracks either tick count, physical clock time, or both).
/// If the execution is interrupted "too early", the analyzer is allowed to return
/// `MaybeItt::InsufficientThinkingTicks` instead of returning a proper analysis.
/// What exactly "too early" means will depend on the particular implementation.
pub trait Analyzer {
    fn set_state(&mut self, state: State);

    /// Repeated calls to `think` will continue from the previous state,
    /// unless `set_state` was called in between.
    /// This allows consumers to make the analyzer think for a while, report an analysis,
    /// think some more, report a (hopefully now-improved) analysis, etc.
    fn think<F>(&mut self, on_tick_complete: F)
    where
        F: FnMut() -> ShouldStopThinking;

    fn evaluation(&self) -> MaybeItt<Evaluation>;

    /// "LOP" stands for "line of play".
    /// If there were Insufficient Thinking Ticks,
    /// this is a no-op that returns `MaybeItt::InsufficientThinkingTicks`.
    fn write_optimal_lop<W>(&self, w: &mut W) -> MaybeItt<()>
    where
        W: ActionWriter;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ShouldStopThinking(pub bool);

#[derive(Debug, Clone, Copy)]
pub enum MaybeItt<T> {
    InsufficientThinkingTicks,
    Ok(T),
}

#[derive(Debug, Clone, Copy)]
pub enum Evaluation {
    MateInN { winner: Player, plies: usize },
    Estimate(EvaluationEstimate),
}
impl PartialOrd for Evaluation {
    /// Conceptually, Alpha-wins-in-N has the evaluation `Infinity - N`,
    /// and Beta-wins-in-N has the evaluation `-Infinity + N`.
    /// In other words, a faster win is better than a slower win.
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.partial_cmp_(other)
    }
}
impl PartialEq for Evaluation {
    fn eq(&self, other: &Self) -> bool {
        self.partial_cmp(other) == Some(Ordering::Equal)
    }
}

/// This is always finite.
/// The more positive, the more favorable for Alpha.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct EvaluationEstimate {
    /// INVARIANT: This must be finite.
    raw: f64,
}
impl EvaluationEstimate {
    pub const fn new(raw: f64) -> Option<Self> {
        if raw.is_finite() {
            return Some(Self { raw });
        }

        None
    }

    pub const fn raw(self) -> f64 {
        self.raw
    }
}
