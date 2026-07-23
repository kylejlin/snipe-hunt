//! Deeper probe of the history-aware seed-2 state after 400 turns.

use std::time::Duration;

use snipe_ai::{SearchConfig, SearchEngine};
use snipe_core::{State, StateData};

fn main() {
    let seconds = argument(1, 30_u64);
    let max_depth = argument(2, 8_u8);
    let state = State::from_data(StateData {
        alpha_animals: [402_047_060, 536_877_344, 1_074_339_849, 128, 0, 0, 0, 0],
        beta_animals: [0, 2, 0, 0, 1_024, 512, 134_217_728, 2_147_483_648],
        snipes: [0, 1, 0, 0, 0, 2, 0, 0],
        side_to_move: 1,
        pending_animal: u8::MAX,
        pending_destination: 0,
    })
    .unwrap();
    let mut engine = SearchEngine::new(SearchConfig {
        time_limit: Duration::from_secs(seconds),
        max_depth,
        selective_move_limit: 0,
        ..SearchConfig::default()
    });
    let result = engine.search(&state);
    println!(
        "RESULT legal={} can_capture={} depth={} score={} nodes={} elapsed={:?} best={:?} pv={:?}",
        state.legal_moves().len(),
        state.has_winning_snipe_capture(),
        result.depth,
        result.score,
        result.stats.nodes + result.stats.qnodes,
        result.stats.elapsed,
        result.best_move,
        result.principal_variation,
    );
}

fn argument<T: std::str::FromStr>(index: usize, default: T) -> T {
    std::env::args()
        .nth(index)
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}
