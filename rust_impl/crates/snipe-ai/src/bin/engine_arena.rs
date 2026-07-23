use std::time::Duration;

use snipe_ai::{play_match, SearchConfig, SearchEngine};
use snipe_core::State;

fn main() {
    let pairs = argument(1, 10_u64);
    let milliseconds = argument(2, 250_u64);
    let first_depth = argument(3, 3_u8);
    let second_depth = argument(4, 4_u8);
    let max_turns = argument(5, 160_usize);

    let config = |depth| SearchConfig {
        time_limit: Duration::from_millis(milliseconds),
        max_depth: depth,
        transposition_table_mb: 64,
        ..SearchConfig::default()
    };
    let mut first = SearchEngine::<State>::new(config(first_depth));
    let mut second = SearchEngine::<State>::new(config(second_depth));
    let positions = (0..pairs).flat_map(|seed| [State::initial(seed), State::initial(seed)]);
    let summary = play_match(positions, &mut first, &mut second, max_turns);

    println!(
        "RESULT pairs={pairs} time_ms={milliseconds} depth_{first_depth}_wins={} depth_{second_depth}_wins={} draws={} avg_turns={:.1}",
        summary.first_wins,
        summary.second_wins,
        summary.draws,
        summary.total_turns as f64 / summary.games() as f64,
    );
}

fn argument<T: std::str::FromStr>(index: usize, default: T) -> T {
    std::env::args()
        .nth(index)
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}
