//! Reproducible randomness for Snipe Hunt applications.
//!
//! This crate is separate from `snipe-core` so the authoritative game-logic
//! crate does not choose how applications seed or randomize games.

use snipe_core::{Animal, InitialStateBuilder, State};

const MAJORS: [Animal; 8] = [
    Animal::Tiger,
    Animal::Dragon,
    Animal::Fish,
    Animal::Elephant,
    Animal::Tiger,
    Animal::Dragon,
    Animal::Fish,
    Animal::Elephant,
];
const MINORS: [Animal; 24] = [
    Animal::Mouse,
    Animal::Ox,
    Animal::Rabbit,
    Animal::Snake,
    Animal::Horse,
    Animal::Ram,
    Animal::Monkey,
    Animal::Rooster,
    Animal::Dog,
    Animal::Boar,
    Animal::Squid,
    Animal::Frog,
    Animal::Mouse,
    Animal::Ox,
    Animal::Rabbit,
    Animal::Snake,
    Animal::Horse,
    Animal::Ram,
    Animal::Monkey,
    Animal::Rooster,
    Animal::Dog,
    Animal::Boar,
    Animal::Squid,
    Animal::Frog,
];

/// Deals a reproducible initial position from a seed.
///
/// This is shared by the browser and self-play trainers so a seed always
/// identifies exactly the same deal everywhere in the project.
pub fn initial_state(seed: u64) -> State {
    let mut majors = MAJORS;
    let mut minors = MINORS;
    let mut rng = seed ^ 0x9E37_79B9_7F4A_7C15;
    shuffle(&mut majors, &mut rng);
    shuffle(&mut minors, &mut rng);

    let mut alpha = [Animal::Mouse; 16];
    alpha[..12].copy_from_slice(&minors[..12]);
    alpha[12..].copy_from_slice(&majors[..4]);
    let mut beta = [Animal::Mouse; 16];
    beta[..12].copy_from_slice(&minors[12..]);
    beta[12..].copy_from_slice(&majors[4..]);
    shuffle(&mut alpha, &mut rng);
    shuffle(&mut beta, &mut rng);

    InitialStateBuilder {
        alpha_reserve: [alpha[0]],
        r1: [alpha[1], alpha[2]],
        r2: alpha[3..15].try_into().expect("fixed slice"),
        r3: [alpha[15]],
        r4: [beta[0]],
        r5: beta[1..13].try_into().expect("fixed slice"),
        r6: [beta[13], beta[14]],
        beta_reserve: [beta[15]],
    }
    .build()
    .expect("two copies of every animal")
}

fn shuffle(deck: &mut [Animal], rng: &mut u64) {
    for index in (1..deck.len()).rev() {
        *rng = splitmix64(*rng);
        deck.swap(index, (*rng as usize) % (index + 1));
    }
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
        assert_animals(first.reserves, Player::Alpha, &[Animal::Rooster]);
        assert_animals(first.r1, Player::Alpha, &[Animal::Mouse, Animal::Rabbit]);
        assert_animals(
            first.r2,
            Player::Alpha,
            &[
                Animal::Mouse,
                Animal::Ox,
                Animal::Tiger,
                Animal::Rabbit,
                Animal::Dragon,
                Animal::Dragon,
                Animal::Snake,
                Animal::Horse,
                Animal::Horse,
                Animal::Monkey,
                Animal::Boar,
                Animal::Fish,
            ],
        );
        assert_animals(first.r3, Player::Alpha, &[Animal::Ram]);
        assert_animals(first.r4, Player::Beta, &[Animal::Frog]);
        assert_animals(
            first.r5,
            Player::Beta,
            &[
                Animal::Ox,
                Animal::Tiger,
                Animal::Snake,
                Animal::Ram,
                Animal::Monkey,
                Animal::Dog,
                Animal::Dog,
                Animal::Fish,
                Animal::Elephant,
                Animal::Squid,
                Animal::Squid,
                Animal::Frog,
            ],
        );
        assert_animals(first.r6, Player::Beta, &[Animal::Rooster, Animal::Elephant]);
        assert_animals(first.reserves, Player::Beta, &[Animal::Boar]);
        assert_eq!(first.r1.count(Card::Snipe, Player::Alpha), 1);
        assert_eq!(first.r6.count(Card::Snipe, Player::Beta), 1);
    }

    #[test]
    fn every_player_gets_exactly_four_major_animals() {
        let major_animals = [
            Animal::Tiger,
            Animal::Dragon,
            Animal::Fish,
            Animal::Elephant,
        ];
        for seed in 0..1_000 {
            let state = initial_state(seed);
            let locations = [
                state.reserves,
                state.r1,
                state.r2,
                state.r3,
                state.r4,
                state.r5,
                state.r6,
            ];
            for player in [Player::Alpha, Player::Beta] {
                let major_count: u32 = locations
                    .into_iter()
                    .map(|cards| {
                        major_animals
                            .into_iter()
                            .map(|animal| u32::from(cards.count(Card::Animal(animal), player)))
                            .sum::<u32>()
                    })
                    .sum();
                assert_eq!(major_count, 4, "seed {seed}, player {player:?}");
            }
        }
    }
}
