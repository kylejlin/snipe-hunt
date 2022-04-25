pub(crate) use internal::*;

mod internal {
    pub use std::collections::HashMap;
    pub use std::hash::Hash;
    pub use std::ops::{AddAssign, Index, IndexMut};
}
