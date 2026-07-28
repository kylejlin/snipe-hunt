//! Reproducible randomness for Snipe Hunt applications.
//!
//! This crate is separate from `snipe-core` so the authoritative game-logic
//! crate does not choose how applications seed or randomize games.

use snipe_core::{Animal, InitialStateBuilder, State};

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

/// Deals a reproducible initial position from a seed.
///
/// This is shared by the browser and self-play trainers so a seed always
/// identifies exactly the same deal everywhere in the project.
pub fn initial_state(seed: u64) -> State {
    let mut deck = [Animal::Mouse; 32];
    for (index, slot) in deck.iter_mut().enumerate() {
        *slot = ANIMALS[index % ANIMALS.len()];
    }
    let mut rng = seed ^ 0x9E37_79B9_7F4A_7C15;
    for index in (1..deck.len()).rev() {
        rng = splitmix64(rng);
        deck.swap(index, (rng as usize) % (index + 1));
    }
    InitialStateBuilder {
        alpha_reserve: [deck[0]],
        r1: [deck[1], deck[2]],
        r2: deck[3..15].try_into().expect("fixed slice"),
        r3: [deck[15]],
        r4: [deck[16]],
        r5: deck[17..29].try_into().expect("fixed slice"),
        r6: [deck[29], deck[30]],
        beta_reserve: [deck[31]],
    }
    .build()
    .expect("two copies of every animal")
}

/// Applies the SplitMix64 mixing function to `value`.
pub const fn splitmix64(mut value: u64) -> u64 {
    value = value.wrapping_add(0x9E37_79B9_7F4A_7C15);
    value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^ (value >> 31)
}

#[cfg(test)]
mod tests {
    use super::*;
    use snipe_core::{Card, CardMultiset, Player};

    fn assert_animals(cards: CardMultiset, player: Player, expected: &[Animal]) {
        for animal in ANIMALS {
            assert_eq!(
                usize::from(cards.count(Card::Animal(animal), player)),
                expected
                    .iter()
                    .filter(|&&candidate| candidate == animal)
                    .count(),
                "{animal:?} count differs"
            );
        }
    }

    #[test]
    fn splitmix64_matches_known_vectors() {
        assert_eq!(splitmix64(0), 0xE220_A839_7B1D_CDAF);
        assert_eq!(splitmix64(1), 0x910A_2DEC_8902_5CC1);
        assert_eq!(splitmix64(u64::MAX), 0xE4D9_7177_1B65_2C20);
    }

    #[test]
    fn an_initial_deal_is_reproducible_and_complete() {
        let first = initial_state(7_071);
        let second = initial_state(7_071);
        let locations = |state: &State| {
            [
                state.reserves,
                state.r1,
                state.r2,
                state.r3,
                state.r4,
                state.r5,
                state.r6,
            ]
        };

        for (left, right) in locations(&first).into_iter().zip(locations(&second)) {
            for animal in [
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
            ] {
                for player in [Player::Alpha, Player::Beta] {
                    assert_eq!(
                        left.count(Card::Animal(animal), player),
                        right.count(Card::Animal(animal), player)
                    );
                }
            }
            for player in [Player::Alpha, Player::Beta] {
                assert_eq!(
                    left.count(Card::Snipe, player),
                    right.count(Card::Snipe, player)
                );
            }
        }

        assert_eq!(first.active_player, Player::Beta);
        assert_eq!(first.leading_action, None);
        assert_animals(first.reserves, Player::Alpha, &[Animal::Boar]);
        assert_animals(
            first.r1,
            Player::Alpha,
            &[Animal::Elephant, Animal::Rooster],
        );
        assert_animals(
            first.r2,
            Player::Alpha,
            &[
                Animal::Dragon,
                Animal::Squid,
                Animal::Monkey,
                Animal::Ox,
                Animal::Ox,
                Animal::Rooster,
                Animal::Ram,
                Animal::Ram,
                Animal::Elephant,
                Animal::Snake,
                Animal::Monkey,
                Animal::Boar,
            ],
        );
        assert_animals(first.r3, Player::Alpha, &[Animal::Tiger]);
        assert_animals(first.r4, Player::Beta, &[Animal::Horse]);
        assert_animals(
            first.r5,
            Player::Beta,
            &[
                Animal::Dog,
                Animal::Mouse,
                Animal::Snake,
                Animal::Rabbit,
                Animal::Dog,
                Animal::Frog,
                Animal::Dragon,
                Animal::Fish,
                Animal::Horse,
                Animal::Fish,
                Animal::Squid,
                Animal::Mouse,
            ],
        );
        assert_animals(first.r6, Player::Beta, &[Animal::Frog, Animal::Tiger]);
        assert_animals(first.reserves, Player::Beta, &[Animal::Rabbit]);
        assert_eq!(first.r1.count(Card::Snipe, Player::Alpha), 1);
        assert_eq!(first.r6.count(Card::Snipe, Player::Beta), 1);
    }
}
