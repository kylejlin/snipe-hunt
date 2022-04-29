pub use external::*;
pub(crate) use internal::*;

mod external {
    pub mod spec {
        pub use crate::spec::{
            action::{Action, Direction},
            state::{Player, Rank, RankIndex, Species, SpeciesMultiset, State},
        };
    }
}

mod internal {
    pub use std::collections::HashMap;
    pub use std::hash::Hash;
    pub use std::ops::{AddAssign, Index, IndexMut};
}
