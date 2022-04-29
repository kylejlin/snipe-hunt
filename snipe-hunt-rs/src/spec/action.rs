use crate::prelude::spec::*;

#[derive(Debug, Clone)]
pub enum Action {
    Animal(RankIndex, Direction, Species),
    Snipe(Direction),
    Drop(RankIndex, Species),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Direction {
    North = 0,
    South = 1,
}
