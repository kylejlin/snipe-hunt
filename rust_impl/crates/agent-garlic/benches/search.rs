use agent_garlic::GarlicAnalyzer;
use snipe_core::Analyzer;
use snipe_prng::initial_state;
use std::{env, hint::black_box, time::Instant};

const DEFAULT_POSITIONS: u64 = 64;
const DEFAULT_TICKS_PER_POSITION: u64 = 64;

fn setting(name: &str, default: u64) -> u64 {
    env::var(name)
        .ok()
        .and_then(|value| value.parse().ok())
        .filter(|&value| value != 0)
        .unwrap_or(default)
}

fn main() {
    let positions = setting("GARLIC_BENCH_POSITIONS", DEFAULT_POSITIONS);
    let ticks_per_position = setting(
        "GARLIC_BENCH_TICKS_PER_POSITION",
        DEFAULT_TICKS_PER_POSITION,
    );
    let started = Instant::now();
    let mut completed_depth = 0u64;

    for seed in 0..positions {
        let mut analyzer = GarlicAnalyzer::new();
        analyzer.set_state(initial_state(seed));
        for _ in 0..ticks_per_position {
            analyzer.think_for_one_tick();
        }
        completed_depth += u64::from(analyzer.completed_depth());
        black_box(analyzer.evaluation());
    }

    let elapsed = started.elapsed();
    let ticks = positions * ticks_per_position;
    println!(
        "{ticks} ticks across {positions} positions in {:.6}s ({:.0} ticks/s, mean completed depth {:.2})",
        elapsed.as_secs_f64(),
        ticks as f64 / elapsed.as_secs_f64(),
        completed_depth as f64 / positions as f64,
    );
}
