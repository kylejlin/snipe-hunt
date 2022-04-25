use crate::prelude::*;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct State {
    turn: Player,
    alpha_reserve: SpeciesMultiset,
    beta_reserve: SpeciesMultiset,
    ranks: [Rank; 6],
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Player {
    Alpha = 0,
    Beta = 1,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SpeciesMultiset([u8; 16]);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Species {
    Rat = 0,
    Ox = 1,
    Tiger = 2,
    Rabbit = 3,
    Dragon = 4,
    Snake = 5,
    Horse = 6,
    Ram = 7,
    Monkey = 8,
    Rooster = 9,
    Dog = 10,
    Boar = 11,
    Fish = 12,
    Elephant = 13,
    Squid = 14,
    Frog = 15,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Rank {
    alpha_snipe_present: bool,
    alpha_animals: SpeciesMultiset,
    beta_snipe_present: bool,
    beta_animals: SpeciesMultiset,
}

impl Index<Species> for SpeciesMultiset {
    type Output = u8;
    fn index(&self, i: Species) -> &Self::Output {
        &self.0[usize::from(i as u8)]
    }
}

impl IndexMut<Species> for SpeciesMultiset {
    fn index_mut(&mut self, i: Species) -> &mut Self::Output {
        &mut self.0[usize::from(i as u8)]
    }
}

impl AddAssign<&SpeciesMultiset> for SpeciesMultiset {
    fn add_assign(&mut self, rhs: &SpeciesMultiset) {
        for (l, r) in self.0.iter_mut().zip(rhs.0.iter()) {
            *l += r;
        }
    }
}
