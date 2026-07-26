//! This module contains the implementation.
//! It does NOT declare any `pub` items (only `pub(crate)`).

use super::*;

impl State {
    // We manually implement this because the public representation combines the
    // reserves, while Debug should display Alpha's above r1 and Beta's below r6.
    pub(crate) fn fmt_(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!()
    }

    pub(crate) fn write_legal_actions_<W>(&self, w: &mut W)
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

impl InitialStateBuilder {
    pub(crate) const fn build_(self) -> Option<State> {
        todo!()
    }
}

impl CardMultiset {
    // We manually implement this so Debug shows the actual cards rather than
    // the private bit-vector representation.
    pub(crate) fn fmt_(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!()
    }

    pub(crate) const fn singleton_(card: Card, allegiance: Player) -> Self {
        todo!()
    }

    pub(crate) const fn count_(self, card: Card, allegiance: Player) -> u8 {
        todo!()
    }

    /// Returns `None` if the sum would have an illegal number of any of the card types.
    /// There can be at most 2 of any animal (regardless of allegiance), at most 1 Alpha Snipe, and at most 1 Beta Snipe.
    pub(crate) const fn checked_add_(self, other: Self) -> Option<Self> {
        todo!()
    }

    /// Returns `None` if the card (of that allegiance) is not present
    pub(crate) const fn remove_one_(self, card: Card, allegiance: Player) -> Option<Self> {
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

    pub(crate) const fn would_activate_triplet_by_entering_(
        self,
        destination: CardMultiset,
    ) -> bool {
        todo!()
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

impl ActionWriter for Vec<Action> {
    fn push(&mut self, action: Action) {
        Vec::push(self, action);
    }

    fn reserve(&mut self, additional: usize) {
        Vec::reserve(self, additional);
    }
}
