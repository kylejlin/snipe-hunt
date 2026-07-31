use agent_avocado::AvocadoAnalyzer;
use agent_garlic::GarlicAnalyzer;
use snipe_core::{Action, Analyzer, Evaluation};
use snipe_prng::initial_state;
use std::{env, time::Duration, time::Instant};

const DEFAULT_POSITIONS: u64 = 32;
const DEFAULT_TARGET_DEPTH: u16 = 4;
const MAX_TICKS: u64 = 1_000_000;

#[derive(Debug, Eq, PartialEq)]
struct ResultAtDepth {
    ticks: u64,
    depth: u16,
    evaluation: Evaluation,
    line: Vec<Action>,
}

fn setting<T>(name: &str, default: T) -> T
where
    T: std::str::FromStr,
{
    env::var(name)
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}

fn run_avocado(seed: u64, target_depth: u16) -> (Duration, ResultAtDepth) {
    let mut analyzer = AvocadoAnalyzer::new();
    analyzer.set_state(initial_state(seed));
    let started = Instant::now();
    let mut ticks = 0;
    while analyzer.completed_depth() < target_depth && analyzer.is_fully_solved().is_none() {
        assert!(
            ticks < MAX_TICKS,
            "Avocado did not reach depth {target_depth}"
        );
        analyzer.think_for_one_tick();
        ticks += 1;
    }
    let elapsed = started.elapsed();
    let mut line = Vec::new();
    analyzer.write_optimal_lop(&mut line);
    (
        elapsed,
        ResultAtDepth {
            ticks,
            depth: analyzer.completed_depth(),
            evaluation: analyzer.evaluation(),
            line,
        },
    )
}

fn run_garlic(seed: u64, target_depth: u16) -> (Duration, ResultAtDepth) {
    let mut analyzer = GarlicAnalyzer::new();
    analyzer.set_state(initial_state(seed));
    let started = Instant::now();
    let mut ticks = 0;
    while analyzer.completed_depth() < target_depth && analyzer.is_fully_solved().is_none() {
        assert!(
            ticks < MAX_TICKS,
            "Garlic did not reach depth {target_depth}"
        );
        analyzer.think_for_one_tick();
        ticks += 1;
    }
    let elapsed = started.elapsed();
    let mut line = Vec::new();
    analyzer.write_optimal_lop(&mut line);
    (
        elapsed,
        ResultAtDepth {
            ticks,
            depth: analyzer.completed_depth(),
            evaluation: analyzer.evaluation(),
            line,
        },
    )
}

fn main() {
    let positions = setting("GARLIC_STRENGTH_POSITIONS", DEFAULT_POSITIONS);
    let target_depth = setting("GARLIC_STRENGTH_TARGET_DEPTH", DEFAULT_TARGET_DEPTH);
    let mut avocado_elapsed = Duration::ZERO;
    let mut garlic_elapsed = Duration::ZERO;
    let mut ticks = 0;

    for seed in 0..positions {
        let (avocado, garlic) = if seed % 2 == 0 {
            let avocado = run_avocado(seed, target_depth);
            let garlic = run_garlic(seed, target_depth);
            (avocado, garlic)
        } else {
            let garlic = run_garlic(seed, target_depth);
            let avocado = run_avocado(seed, target_depth);
            (avocado, garlic)
        };
        avocado_elapsed += avocado.0;
        garlic_elapsed += garlic.0;
        ticks += garlic.1.ticks;
        assert_eq!(
            garlic.1, avocado.1,
            "Garlic diverged from Avocado at equal depth for seed {seed}"
        );
    }

    println!(
        "matched {positions} positions exactly through depth {target_depth} ({ticks} ticks): Avocado {:.6}s, Garlic {:.6}s, {:.2}x faster",
        avocado_elapsed.as_secs_f64(),
        garlic_elapsed.as_secs_f64(),
        avocado_elapsed.as_secs_f64() / garlic_elapsed.as_secs_f64(),
    );
}
