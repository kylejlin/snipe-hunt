//! This crate meant to be an authoritative reference implementation.
//! It is NOT time-efficient or space-efficient.
//! Instead, it prioritizes a clean interface and developer ergonomics.
//!
//! The web UI (which shouldn't be too computationally intensive) should use this crate as much as possible.
//! However, the analyzers (which _extremely_ computationally intensive) should only use
//! this crate at public interface boundaries, while internally using more efficient algorithms/data-structures.

pub use std::cmp::Ordering;

/// This module contains the implementation.
/// It does NOT declare any `pub` items.
mod impl_;

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
/// Also, it allows an `incomplete_ply`, which further improves developer ergonomics
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
    pub incomplete_ply: Option<IncompletePly>,
}

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
impl CardMultiset {
    pub const EMPTY: Self = Self {
        alpha_presence: 0,
        beta_presence: 0,
        has_allied_twins: 0,
        snipes: 0,
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IncompletePly {
    FirstAnimalStep(AnimalStep),
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
        todo!()
    }

    pub const fn unary_element(self) -> Option<Element> {
        todo!()
    }

    pub const fn binary_element(self) -> Option<Element> {
        todo!()
    }

    pub const fn ternary_element(self) -> Option<Element> {
        Some(match self {
            Self::Tiger => Element::Fire,
            Self::Dragon => Element::Air,
            Self::Fish => Element::Water,
            Self::Elephant => Element::Earth,
            _ => return None,
        })
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
pub enum Ply {
    AnimalSteps(AnimalStep, AnimalStep),
    SnipeStep,
    Drop(AnimalDrop),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AnimalDrop {
    pub actor: Animal,
    pub destination: Rank,
}

pub trait Analyzer {
    fn set_state(&mut self, state: State);

    fn think<F>(&mut self, on_tick: F)
    where
        F: FnMut() -> ShouldStopThinking;

    fn evaluation(&self) -> MaybeIta<Evaluation>;

    fn write_optimal_lop<W>(&self, w: W) -> MaybeIta<()>
    where
        W: LopWriter;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ShouldStopThinking(pub bool);

#[derive(Debug, Clone, Copy)]
pub enum MaybeIta<T> {
    InsufficientThinkingAllowance,
    Ok(T),
}

/// "LOP" stands for "line of play".
pub trait LopWriter {
    fn push_second_animal_step(&mut self, step: AnimalStep);

    fn push_ply(&mut self, ply: Ply);

    fn reserve(&mut self, additional_ply_count: usize);
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
        Some(match (self, other) {
            (
                Self::MateInN {
                    winner: Player::Beta,
                    plies: _,
                },
                Self::Estimate(_),
            )
            | (
                Self::Estimate(_),
                Self::MateInN {
                    winner: Player::Alpha,
                    plies: _,
                },
            )
            | (
                Self::MateInN {
                    winner: Player::Beta,
                    plies: _,
                },
                Self::MateInN {
                    winner: Player::Alpha,
                    plies: _,
                },
            ) => Ordering::Less,

            (
                Self::Estimate(_),
                Self::MateInN {
                    winner: Player::Beta,
                    plies: _,
                },
            )
            | (
                Self::MateInN {
                    winner: Player::Alpha,
                    plies: _,
                },
                Self::Estimate(_),
            )
            | (
                Self::MateInN {
                    winner: Player::Alpha,
                    plies: _,
                },
                Self::MateInN {
                    winner: Player::Beta,
                    plies: _,
                },
            ) => Ordering::Greater,

            (
                Self::MateInN {
                    winner: Player::Alpha,
                    plies: left,
                },
                Self::MateInN {
                    winner: Player::Alpha,
                    plies: right,
                },
            ) => left.cmp(right).reverse(),

            (
                Self::MateInN {
                    winner: Player::Beta,
                    plies: left,
                },
                Self::MateInN {
                    winner: Player::Beta,
                    plies: right,
                },
            ) => left.cmp(right),

            (Self::Estimate(left), Self::Estimate(right)) => left.partial_cmp(right)?,
        })
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
