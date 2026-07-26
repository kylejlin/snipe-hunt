use std::time::Duration;

use snipe_ai::{SearchConfig, SearchEngine};
use snipe_core::State;

fn main() {
    let seeds = argument(1, 10_u64);
    let milliseconds = argument(2, 5_000_u64);
    let max_depth = argument(3, 8_u8);

    let mut depths = Vec::with_capacity(seeds as usize);
    let mut total_nodes = 0_u64;
    let mut total_ms = 0_u128;
    for seed in 0..seeds {
        let mut engine = SearchEngine::<State>::new(SearchConfig {
            time_limit: Duration::from_millis(milliseconds),
            max_depth,
            transposition_table_mb: 64,
            ..SearchConfig::default()
        });
        let result = engine.search(&State::initial(seed));
        let nodes = result.stats.nodes + result.stats.qnodes;
        println!(
            "seed={seed:>2} depth={} nodes={nodes:>9} elapsed_ms={:>5} score={:>5}",
            result.depth,
            result.stats.elapsed.as_millis(),
            result.score,
        );
        depths.push(result.depth);
        total_nodes += nodes;
        total_ms += result.stats.elapsed.as_millis();
    }

    depths.sort_unstable();
    println!(
        "RESULT seeds={seeds} budget_ms={milliseconds} min_depth={} median_depth={} max_depth={} avg_nodes={:.0} avg_ms={:.0}",
        depths.first().copied().unwrap_or(0),
        depths.get(depths.len() / 2).copied().unwrap_or(0),
        depths.last().copied().unwrap_or(0),
        total_nodes as f64 / seeds.max(1) as f64,
        total_ms as f64 / seeds.max(1) as f64,
    );
}

fn argument<T: std::str::FromStr>(index: usize, default: T) -> T {
    std::env::args()
        .nth(index)
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}
