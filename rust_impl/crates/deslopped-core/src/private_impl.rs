//! This module contains the implementation.
//! It does NOT declare any `pub` items (only `pub(crate)`).

use super::*;

impl State {
    pub(crate) fn write_legal_actions_<W>(&self, w: W)
    where
        W: ActionWriter,
    {
        todo!()
    }

    pub(crate) fn apply_(self, action: Action) -> Result<Self, IllegalActionError> {
        todo!()
    }

    pub(crate) fn winner_(&self) -> Option<Player> {
        todo!()
    }
}

impl Animal {
    pub(crate) const fn is_retreater_(self) -> bool {
        todo!()
    }

    pub(crate) const fn unary_element_(self) -> Option<Element> {
        todo!()
    }

    pub(crate) const fn binary_element_(self) -> Option<Element> {
        todo!()
    }

    pub(crate) const fn ternary_element_(self) -> Option<Element> {
        Some(match self {
            Self::Tiger => Element::Fire,
            Self::Dragon => Element::Air,
            Self::Fish => Element::Water,
            Self::Elephant => Element::Earth,
            _ => return None,
        })
    }
}

impl Evaluation {
    /// Conceptually, Alpha-wins-in-N has the evaluation `Infinity - N`,
    /// and Beta-wins-in-N has the evaluation `-Infinity + N`.
    /// In other words, a faster win is better than a slower win.
    pub(crate) fn partial_cmp_(&self, other: &Self) -> Option<Ordering> {
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
