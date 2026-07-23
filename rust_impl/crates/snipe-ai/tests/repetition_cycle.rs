use std::time::Duration;

use snipe_ai::{SearchConfig, SearchEngine};
use snipe_core::{Animal, AnimalStep, Move, Row, State, StateData};

/// Production depth-three self-play from seed 3 first reached this state on
/// turn 85, then returned to it exactly on turn 101. The game has no repetition
/// draw rule, so choosing this loop indefinitely is not a completed game.
fn cycle_position() -> State {
    State::from_data(StateData {
        alpha_animals: [
            1_787_365_385,
            0,
            0,
            0,
            2_097_184,
            67_137_536,
            2_432_862_020,
            0,
        ],
        beta_animals: [0, 1_310_738, 128, 0, 0, 0, 0, 4_194_304],
        snipes: [0, 0, 2, 1, 0, 0, 0, 0],
        side_to_move: 0,
        pending_animal: u8::MAX,
        pending_destination: 0,
    })
    .unwrap()
}

#[test]
fn production_self_play_cycle_is_legal_and_returns_to_identical_state() {
    let initial = cycle_position();
    assert_eq!(initial.position_hash(), 0xa335_36c9_fdf9_441c);

    let cycle = cycle_moves();

    let mut state = initial;
    for (index, mv) in cycle.into_iter().enumerate() {
        assert!(
            state.legal_moves().contains(&mv),
            "cycle move {index} is illegal: {mv:?}"
        );
        state = state.apply_move(mv).unwrap();
        assert!(
            state.winner().is_none(),
            "cycle unexpectedly ended after move {index}"
        );
    }
    assert_eq!(state, initial);
}

#[test]
fn historical_contempt_avoids_closing_the_production_cycle() {
    let repeated = cycle_position();
    let cycle = cycle_moves();
    let mut before_closing_line = repeated;
    for mv in cycle[..cycle.len() - 2].iter().copied() {
        before_closing_line = before_closing_line.apply_move(mv).unwrap();
    }
    let closing_setup = cycle[cycle.len() - 2];
    let forced_closure = cycle[cycle.len() - 1];
    assert_eq!(
        before_closing_line
            .apply_move(closing_setup)
            .unwrap()
            .apply_move(forced_closure)
            .unwrap(),
        repeated
    );

    let config = SearchConfig {
        time_limit: Duration::from_secs(5),
        max_depth: 3,
        ..SearchConfig::default()
    };
    let mut without_history = SearchEngine::new(config.clone());
    let baseline = without_history.search(&before_closing_line);
    assert_eq!(baseline.depth, 3, "{baseline:?}");
    assert_eq!(
        baseline.best_move,
        Some(closing_setup),
        "fixture must reproduce the original production cycle"
    );

    let mut with_history = SearchEngine::new(config);
    let defended =
        with_history.search_with_history(&before_closing_line, &[repeated.repetition_hash()]);
    assert_eq!(defended.depth, 3, "{defended:?}");
    let defended_move = defended.best_move.expect("position is nonterminal");
    assert_ne!(defended_move, closing_setup, "{defended:?}");
}

#[test]
fn seed_two_wandering_cycle_is_identical_modulo_copy_labels() {
    let first = State::from_data(StateData {
        alpha_animals: [1_476_372_800, 536_875_060, 16_393, 128, 0, 0, 0, 0],
        beta_animals: [0, 2, 0, 0, 1_024, 512, 134_217_728, 2_147_483_648],
        snipes: [0, 1, 0, 0, 2, 0, 0, 0],
        side_to_move: 1,
        pending_animal: u8::MAX,
        pending_destination: 0,
    })
    .unwrap();
    let repeated = State::from_data(StateData {
        alpha_animals: [1_467_918_785, 536_875_060, 81_928, 8_388_608, 0, 0, 0, 0],
        beta_animals: [0, 2, 0, 0, 1_024, 512, 134_217_728, 2_147_483_648],
        snipes: [0, 1, 0, 0, 2, 0, 0, 0],
        side_to_move: 1,
        pending_animal: u8::MAX,
        pending_destination: 0,
    })
    .unwrap();

    assert_ne!(first.position_hash(), repeated.position_hash());
    assert_eq!(first.repetition_hash(), repeated.repetition_hash());
}

fn cycle_moves() -> [Move; 16] {
    [
        animals(Animal::Squid1, Row::Four, Animal::Snake1, Row::Five),
        Move::Snipe {
            destination: Row::One,
        },
        drop(Animal::Fish1, Row::Five),
        Move::Snipe {
            destination: Row::Two,
        },
        drop(Animal::Dog2, Row::Five),
        Move::Snipe {
            destination: Row::One,
        },
        drop(Animal::Elephant1, Row::Five),
        Move::Snipe {
            destination: Row::Two,
        },
        animals(Animal::Snake1, Row::Four, Animal::Squid1, Row::Five),
        Move::Snipe {
            destination: Row::One,
        },
        drop(Animal::Fish1, Row::Five),
        Move::Snipe {
            destination: Row::Two,
        },
        drop(Animal::Dog2, Row::Five),
        Move::Snipe {
            destination: Row::One,
        },
        drop(Animal::Elephant1, Row::Five),
        Move::Snipe {
            destination: Row::Two,
        },
    ]
}

fn animals(first: Animal, first_row: Row, second: Animal, second_row: Row) -> Move {
    Move::Animals {
        first: AnimalStep {
            moved: first,
            destination: first_row,
        },
        second: Some(AnimalStep {
            moved: second,
            destination: second_row,
        }),
    }
}

fn drop(animal: Animal, destination: Row) -> Move {
    Move::Drop {
        animal,
        destination,
    }
}
